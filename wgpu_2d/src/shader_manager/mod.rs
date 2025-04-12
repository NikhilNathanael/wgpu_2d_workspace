use crate::wgpu_context::WGPUContext;
use std::collections::{HashMap, HashSet};
use std::fs::read_to_string;
use std::io::ErrorKind;
use wgpu::*;

use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::num::NonZeroU32;
use std::sync::Mutex;

/// Manages loading and compilation of shaders from disk
///
/// Uses unsafe code to allow taking shared references into the data while
/// mutating the HashMaps. This use of unsafe has been thought through but
/// it has not been fully verified. Specific safety comments can be found
/// at the site of the unsafe blocks
///
/// Shared references to the shader modules and render pipelines can be obtained
/// with the same lifetime as the shared reference to self instead of to the MutexGuard.
/// This should be okay as long as
/// 	a) The actual heap allocation of the box is never moved or modified
/// 	b) The hashmap never removes or replaces a shader module or render pipeline through
/// 	   a shared reference
///
/// Mutable references are allowed to modify the data in any way they want. This is used to
/// clear the data to allow for hot-reloading
///
/// Unanswered questions about safety
///	- The code below creates mutable references to the box (`&mut Box<T>`). Care is taken to
///   avoid a shared reference to the actual box (i.e. `&Box<T>`) as that would immediately be UB.
///	  However, the returned reference (`&'a T`) is derived from such a shared reference. It is
/// 	  unclear if this is also UB
///
///	- A simplified version of the shader manager which requires no OS code (file opening)
///	  was tested with miri in the test module below. It detected no UB in the current configuration
///		- A sanity check was performed by returning a static refernce from get_module and it
/// 		  correctly identified the UB
/// - This seems to suggest that this use of unsafe is indeed sound, but further research is needed
///

/// # TODO: 
/// - Change Mutexes in the fields to RwLock and upgrade Reader Locks to Writer Locks 
/// when needed
/// - Change all these panics to return a result instead
/// - Replace all manual uses of unsafe to use the new unsafe extend_lifetime function instead
/// - add support for shaders stored in the binary itself. 
/// 	- Added with add_literal
/// 	- includes are not resolved here
/// 	- have an associated path 
/// 	- any resolve operation checks these shaders and the filesystem
/// 		- conflicts result in panic

pub struct ShaderManager {
	/// Directory to search for dynamic shaders
    directory_path: Box<str>,
	/// File contents cached in memory keyed by relative path to file.
	///
	/// These may or may not be complete or valid shader files. 
	/// A shader module may include multiple files before being 
	/// compiled.
	///
	/// The paths here MUST be mutually exclusive to the paths in 
	/// [Self::constant_source_files]
	///
	/// These are removed by [Self::reload] 
    source_files: Mutex<HashMap<Box<str>, Box<str>>>,
	/// Stores Shader source files that are not stored on the disk
	/// but are stored within the final binary
	///
	/// These may or may not be complete or valid shader files. 
	/// A shader module may include multiple files before being 
	/// compiled.
	///
	/// The paths here MUST be mutually exclusive to the paths in 
	/// [Self::source_files]
	///
	/// These are not removed when [Self::reload] is called
	constant_source_files: Mutex<HashMap<Box<str>, Box<str>>>,
	/// Cached [ShaderModule]s
	///
	/// [ShaderModule]s are returned from here if available
    shader_modules: Mutex<HashMap<Box<str>, Box<ShaderModule>>>,
	/// Cached [RenderPipeline]s 
	///
	/// [RenderPipeline]s are returned from here if available
    render_pipelines: Mutex<
        HashMap<
            Box<str>,
            (
                RenderPipelineDescriptorTemplate,
                Option<Box<RenderPipeline>>,
            ),
        >,
    >,
}

impl ShaderManager {
	/// Creates a new [ShaderManager]
    pub fn new(directory_path: &str) -> Self {
        Self {
            directory_path: directory_path.into(),
            source_files: Mutex::new(HashMap::new()),
			constant_source_files: Mutex::new(HashMap::new()),
            shader_modules: Mutex::new(HashMap::new()),
            render_pipelines: Mutex::new(HashMap::new()),
        }
    }

	/// Searches [Self::source_files] for the given path and returns it if present
	/// or tries to read it from disk and if found, caches and returns it
	///
	fn get_file_from_disk<'a>(&'a self, path: &str) -> Option<&'a str> {
		let mut lock = self.source_files.lock().unwrap();
		match lock.get(path) {
			// SAFETY: See extend_lifetime
			Some(file) => return Some(unsafe{extend_lifetime(&**file)}),
			None => (),
		}
		match read_to_string(self.directory_path.to_string() + &*path) {
			Ok(file) => {
				let file = lock.entry(path.into()).or_insert(file.into());
				// SAFETY: See extend_lifetime
				Some(unsafe{extend_lifetime(&**file)})
			}
			Err(err) if err.kind() == ErrorKind::NotFound => {
				None
			}
			Err(err) => {
				panic!("Error while attempting to read file with path {} {err:?}", path);
			}
		}
	}

	/// Searches [Self::constant_source_files] for the given path and returns it if present
	fn get_file_from_constant_source<'a>(&'a self, path: &str) -> Option<&'a str> {
		let lock = self.constant_source_files.lock().unwrap();
		match lock.get(path) {
			// SAFETY: See extend_lifetime
			Some(file) => return Some(unsafe{extend_lifetime(&**file)}),
			None => None,
		}
	}

	/// Gets the source file and then iteratively expands each of the include statements
	fn get_source_new<'a>(&'a self, path: &str) -> String {
		// At this point, we know the shader source is not cached
        log::debug!("source file not already loaded: {:?}", path);

		// Check if file has been loaded from disk or is a constant source
		let disk_source_file = self.get_file_from_disk(path);
		let const_source_file = self.get_file_from_constant_source(path);

		let mut source = match (disk_source_file, const_source_file) {
			(Some(source), None) | (None, Some(source)) => source,
			// If both return a source file or neither return one, then panic
			(Some(_), Some(_)) => {
				panic!("Requested shader path {} is available on disk and in constant shaders", path);
			}
			(None, None) => {
				panic!("Requested shader path {} not found on disk or in constant shaders", path);
			}
		}.to_string();

		let mut includes: HashSet<Box<str>> = HashSet::new();

		// - While there is a next include file
		// 		- check that path isnt already included
		// 		- add the include path to set
		// 		- insert source at location
		// 		- repeat

		while let Some((line, include)) = find_next_include(&source) {
			if !includes.insert(include.into()) {
				panic!("Include path {} already seen when processing file {}", include, path);
			}
			// create string slice from start of string to beginning of line with include
			//
			// get source file of include path 
			//
			// create string slice from end of line with include and end of string
			//
			// concatenate all three slices

			let first = {
				// SAFETY: line is derived from source and is guaranteed by safe code to be within 
				// valid range for source
				let offset = unsafe {
					source.as_ptr().offset_from(line.as_ptr())
				}.try_into().expect("line must be after or equal to source start");
				// SAFETY: 
				// - start is the guaranteed to be the start of a utf-8 char it is equal to the start of source
				// - end is guaranteed to be the end of a utf-8 char because it is one before the start of line
				// - all bytes in this slice are guaranteed to be valid utf-8 because it comes from a source
				unsafe{
					std::str::from_utf8_unchecked(
						// SAFETY: slice between start of source and start of line with include is guaranteed to be valid
						std::slice::from_raw_parts(source.as_ptr(), offset)
					)
				}
			};
			let middle = {
				// Check if file has been loaded from disk or is a constant source
				let disk_source_file = self.get_file_from_disk(include);
				let const_source_file = self.get_file_from_constant_source(include);

				match (disk_source_file, const_source_file) {
					(Some(source), None) | (None, Some(source)) => source,
					// If both return a source file or neither return one, then panic
					(Some(_), Some(_)) => {
						panic!("Requested shader path {} is available on disk and in constant shaders", path);
					}
					(None, None) => {
						panic!("Requested shader path {} not found on disk or in constant shaders", path);
					}
				}
			};
			let last = {
				// get offset from start of source and end of line and 
				// create a pointer with that offset from start of source
				// 
				// Note: This pointer CANNOT be created from pointer of line
				// because line DOES NOT have provenance over the entire string
				//
				// get distance from end of line to end of source
				//
				// create new string slice
				
				let start_offset: usize = usize::try_from(unsafe {
					source.as_ptr().offset_from(line.as_ptr())
				}).expect("line must be after or equal to source start")
				+ line.len();

				let len = source.len() - start_offset;

				// SAFETY: 
				// - start is the guaranteed to be the start of a utf-8 char it is one past the end of line
				// - end is guaranteed to be the end of a utf-8 char because it is the same as the end of source
				// - all bytes in this slice are guaranteed to be valid utf-8 because it comes from a source
				unsafe{
					std::str::from_utf8_unchecked(
						// SAFETY: slice between start of source and start of line with include is guaranteed to be valid
						std::slice::from_raw_parts(
							// SAFETY: start_offset is guaranteed to be within source
							source.as_ptr().add(start_offset),
							len
						)
					)
				}
			};

			source = first.to_string() + middle + last;
		}

		return source;

		// Go line by line and find the first line that contains an include directive
		// if its present
		fn find_next_include(input: &str) -> Option<(&str, &str)> {
			input.lines().enumerate().filter_map(|(i, line)| {
				let path_container = line.trim().split_once("#include")?.1.trim();
				(|| {
					Some((line, path_container.split_once('<')?.1.rsplit_once('>')?.0))
				})()
				.or(None)
			}).next()
		}
	}

	/// Calls [Self::get_source] and creates a [ShaderModule] from the returned source
    fn read_and_get_module(&self, path: &str, context: &WGPUContext) -> ShaderModule {
		// - Get source string
		// - Create Shader Module
        let file = Cow::Owned(self.get_source_new(path));
        context
            .device()
            .create_shader_module(ShaderModuleDescriptor {
                label: Some(path),
                source: ShaderSource::Wgsl(file),
            })
    }

	/// Internal API for resolving a [ShaderModule] or returning an existing
	/// [ShaderModule]
    fn get_module<'a>(&'a self, path: &str, context: &WGPUContext) -> &'a ShaderModule {
        // SAFETY: The only thing that can invalidate the lifetime of the returned reference
        // is if the backing Box is deallocated (moving a box does not invalidate pointers into it)
        //
        // The returned reference's lifetime is tied to the shared borrow of self and we do not
        // allow any operations with a shared reference to self to drop or remove any element
        // from the map
        unsafe {
            &*(&**self.shader_modules.lock().unwrap()
			.entry(path.into())
			// putting a new module into the map could invalidate old references 
			// but we ensure that this is never done to an existing module
			.or_insert(Box::new(self.read_and_get_module(path, context)))

			// BE VERY CAREFUL ADDING ANY EXTRA LINES OF CODE HERE
			as *const ShaderModule)
        }
    }

	/// Called the first time a [RenderPipeline] with a specific label is requested after 
	/// a reload. 
    fn compile_pipeline(
        &self,
        template: &RenderPipelineDescriptorTemplate,
        context: &WGPUContext,
    ) -> RenderPipeline {
		// - Get paths from paths from the templates
		// - Get the modules
		// - Create the pipeline descriptor
		// - Compile it
        let paths = template.get_module_paths();
        let modules = (
            self.get_module(paths.0, context),
            paths.1.map(|x| self.get_module(x, context)),
        );
        let descriptor = template.resolve(modules.0, modules.1);

        context.device().create_render_pipeline(&descriptor)
    }

	/// Returns an already compiled pipeline with the [RenderPipelineDescriptor] template 
	/// registered with the given label.
	///
	/// If such a pipeline does not exist yet, compile one using the given template
    pub fn get_render_pipeline<'a>(
        &'a self,
        label: &str,
        context: &WGPUContext,
    ) -> &'a RenderPipeline {
        match self
            .render_pipelines
            .lock()
            .unwrap()
            .get_mut(label)
			// TODO: Change this expect to a pattern match inside the match block with 
			// a proper error message
            .expect("Tried to access a render pipeline that wasn't registered")
        {
            // SAFETY: The only thing that can invalidate the lifetime of the returned reference
            // is if the backing Box is deallocated (moving a box does not invalidate pointers into it)
            //
            // The returned reference's lifetime is tied to the shared borrow of self and we do not
            // allow any operations with a shared reference to self to drop or remove an element
            // from the map
            (template, x) => unsafe {
                &*(
                    // putting a new pipeline into the map could invalidate old references,
                    // but we ensure that this is only done if there wasn't already a pipeline there
                    &**x.get_or_insert_with(
						// BE VERY CAREFUL ADDING ANY EXTRA LINES OF CODE HERE
                        || Box::new(self.compile_pipeline(template, context)), 
                    ) as *const RenderPipeline
                )
            },
        }
    }

	/// Registers a specific [RenderPipelineDescriptorTemplate] with a label.
	/// Not reset when reload is called
    pub fn register_render_pipeline(
        &self,
        label: &str,
        template: RenderPipelineDescriptorTemplate,
    ) {
		// TODO: Change this to only allocate the label if insertion is necessary
        match self.render_pipelines.lock().unwrap().entry(label.into()) {
            // we only have shared access to self here so there may be borrows into
            // any existing pipeline here.
            // we must take care not to remove any existing render pipelines
            Entry::Occupied(_) => (),

            // this insertion is fine because there is not render pipeline to
            // invalidate here
            Entry::Vacant(x) => {
                x.insert((template, None));
            }
        }
    }
	
	/// Registers a new constant shader source file. This is intended for source 
	/// files which are included in the binary which cannot be obtained again after a reload
	/// 
	/// *Note*: Shader source is not verified here, but rather when [Self::get_render_pipeline] 
	/// is called
	///
	/// # Panics
	/// When a shader source was already registered at this path but the old contents 
	/// do not match the new contents
	///
	/// # Question
	/// Should this return a result to indicate an error instead of panicking
	pub fn register_constant_source(&self, path: &str, source: Box<str>) {
		let mut lock = self.constant_source_files
			.lock().unwrap();
		match lock.get(path) {
			Some(old_source) if *old_source == source => (),
			Some(old_source) => {
				panic!("Conflicting source files registered at path {}: \n\n Old Source : {} \n\n New Source: {} \n\n",
					path,
					old_source,
					source,
				);
			}
			None => {lock.insert(path.into(), source);},
		}
	}

	/// Remove all resolved shaders and pipelines
    pub fn reload(&mut self) {
        // These mutable operations are fine because we have mutable access to self
        // so there are no borrows of this data
		// TODO: Change these locks to get_mut
        self.source_files.lock().unwrap().clear();
        self.shader_modules.lock().unwrap().clear();
        self.render_pipelines
            .lock()
            .unwrap()
            .iter_mut()
            .for_each(|(_, (_, x))| *x = None);
    }
}

/// This is intended for use in ShaderManager to extend the lifetimes of the shader 
/// source and shader modules to the lifetime of the ShaderModule reference instead 
/// of the [Mutex] lock obtained within each function.
///
/// Safety comments at each call location will refer here instead of providing 
/// justification at callsite
unsafe fn extend_lifetime<'a, 'b, T: ?Sized>(input: &'a T) -> &'b T {
	unsafe {&*(input as *const T)}
}

/// A template that can be used to instantiate a [`RenderPipelineDescriptor`]
#[derive(Debug, Clone, PartialEq)]
pub struct RenderPipelineDescriptorTemplate {
	/// Corresponds to [`RenderPipelineDescriptor::label`]
    pub label: Label<'static>,
	/// Corresponds to [`RenderPipelineDescriptor::layout`]
    pub layout: Option<PipelineLayout>,
	/// Template version of [`RenderPipelineDescriptor::vertex`] 
    pub vertex: VertexStateTemplate,
	/// Corresponds to [`RenderPipelineDescriptor::primitive`]
    pub primitive: PrimitiveState,
	/// Corresponds to [`RenderPipelineDescriptor::depth_stencil`]
    pub depth_stencil: Option<DepthStencilState>,
	/// Corresponds to [`RenderPipelineDescriptor::multisample`]
    pub multisample: MultisampleState,
	/// Template version of [`RenderPipelineDescriptor::fragment`]
    pub fragment: Option<FragmentStateTemplate>,
	/// Corresponds to [`RenderPipelineDescriptor::multiview`]
    pub multiview: Option<NonZeroU32>,
	/// Corresponds to [`RenderPipelineDescriptor::cache`]
    pub cache: Option<&'static PipelineCache>,
}

impl RenderPipelineDescriptorTemplate {
	/// Creates a [RenderPipelineDescriptor] to use during shader compilation
	///
	/// Calls [VertexStateTemplate::resolve] with v_module and 
	/// [FragmentStateTemplate::resolve] with f_module to get 
	/// a [VertexState] and [FragmentState]
    fn resolve<'a>(
        &'a self,
        v_module: &'a ShaderModule,
        f_module: Option<&'a ShaderModule>,
    ) -> RenderPipelineDescriptor<'a> {
        RenderPipelineDescriptor {
            label: self.label,
            layout: self.layout.as_ref(),
            vertex: self.vertex.resolve(v_module),
            primitive: self.primitive,
            depth_stencil: self.depth_stencil.clone(),
            multisample: self.multisample,
            fragment: self.fragment.as_ref().map(|x| x.resolve(f_module.unwrap())),
            multiview: self.multiview,
            cache: self.cache,
        }
    }

	/// Calls [VertexStateTemplate::get_module_path] and [FragmentStateTemplate::get_module_path] 
	/// and returns them as a tuple 
	///
	/// The Fragment path is in an [Option] because [FragmentState] is 
	/// optional in [RenderPipelineDescriptor]
    fn get_module_paths(&self) -> (&'static str, Option<&'static str>) {
        (
            self.vertex.get_module_path(),
            self.fragment.as_ref().map(|x| x.get_module_path()),
        )
    }
}

/// A template that can be used to instantiate a [VertexState]
///
/// This does not support overridable constants so [VertexState::compilation_options] does 
/// not have an equivalent here
#[derive(Debug, Clone, PartialEq)]
pub struct VertexStateTemplate {
	/// The path of the shader file relative to the shader source of the [ShaderManager] this gets passed to
	/// 
	/// This is the difference between [VertexStateTemplate] and [VertexState]
    pub module_path: &'static str,
	/// Corresponds to [VertexState::entry_point]
    pub entry_point: Option<&'static str>,
	/// Corresponds to [VertexState::buffers]
    pub buffers: &'static [VertexBufferLayout<'static>],
}

impl VertexStateTemplate {
	/// Creates a [FragmentState] to use during shader compilation
	/// 
	/// The template module path is replaced with the module parameter.
	///
	/// The caller is responsible for ensuring the correct module is passed
    fn resolve<'a>(&self, module: &'a ShaderModule) -> VertexState<'a> {
        VertexState {
            module,
            entry_point: self.entry_point,
            // We do not support overridable constants here
            compilation_options: Default::default(),
            buffers: self.buffers,
        }
    }

	/// Getter for [Self::module_path]
    fn get_module_path(&self) -> &'static str {
        self.module_path
    }
}

/// A template that can be used to instantiate a [FragmentState]
///
/// This does not support overridable constants so [FragmentState::compilation_options] does 
/// not have an equivalent here
#[derive(Debug, Clone, PartialEq)]
pub struct FragmentStateTemplate {
	/// The path of the shader file relative to the shader source of the [ShaderManager] this gets passed to
	/// 
	/// This is the difference between [FragmentStateTemplate] and [FragmentState]
    pub module_path: &'static str,
	/// Corresponds to [FragmentState::entry_point]
    pub entry_point: Option<&'static str>,
	/// Corresponds to [FragmentState::targets]
    pub targets: Box<[Option<ColorTargetState>]>,
}

impl FragmentStateTemplate {
	/// Creates a [FragmentState] to use during shader compilation
	///
	/// The template module path is replaced with the module parameter.
	///
	/// The caller is responsible for ensuring the correct module is passed
    fn resolve<'a>(&'a self, module: &'a ShaderModule) -> FragmentState<'a> {
        FragmentState {
            module,
            entry_point: self.entry_point,
            // We do not support overridable constants here
            compilation_options: Default::default(),
            targets: &self.targets,
        }
    }

	/// Getter for [Self::module_path]
    fn get_module_path(&self) -> &'static str {
        self.module_path
    }
}
