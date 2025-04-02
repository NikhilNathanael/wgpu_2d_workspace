// use any_map::AnyMap;

use std::any::{Any, TypeId};
use std::collections::HashMap;

type AnyMap = HashMap<TypeId, Box<dyn Any>>;

pub struct Scheduler {
	schedule: HashMap<Schedule, Vec<Box<dyn System>>>,
	resources: Resources,
	entities: Entities,
}

impl Scheduler {
	pub fn new () -> Self {
		Self {
			schedule: Default::default(),
			resources: Default::default(),
			entities: Default::default(),
		}
	}

	pub fn add_system<S: IntoSystem<Input>, Input>(&mut self, schedule: Schedule, system: S) where
		S::System: System + 'static,
	{
		self.schedule.entry(schedule)
			.or_insert(vec![])
			.push(Box::new(system.into_system()));
	}

	pub fn add_resource<R: Resource + 'static> (&mut self, res: R) {
		self.resources.add_resource(res);
	}

	pub fn run_schedule(&mut self, schedule: Schedule) {
		for system in self.schedule.get_mut(&schedule).into_iter().flatten() {
			system.run(&self.resources, &self.entities);
		}
	}
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Schedule {
	Startup,
	Update,
	Render,
}

pub struct FunctionSystem<Input, F> {
	f: F,
	marker: std::marker::PhantomData<fn() -> Input>,
}

pub struct Chain<F0, F1> {
	first_finished: bool,
	f0: F0,
	f1: F1,
}

pub struct Set<F> {
	set: F,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveError {
	NotFound,
	ReadError,
	WriteError,
}

pub trait SystemInput: Sized {
	type Item <'new>;
	fn resolve<'r>(resources: &'r Resources, entities: &'r Entities) -> Result<Self::Item<'r>, ResolveError>;
}

pub trait System {
	fn run(&mut self, resources: &Resources, entities: &Entities) -> Result<(), ResolveError>;
}

pub trait IntoSystem<Input> {
	type System: System;
	fn into_system(self) -> Self::System;
	fn chain<Other: IntoSystem<I1>, I1>(self, other: Other) -> Chain<Self::System, Other::System> where Self: Sized {
		Chain {
			first_finished: false,
			f0: self.into_system(),
			f1: other.into_system(),
		}
	}
}

impl<F0: System, F1: System> IntoSystem<()> for Chain<F0, F1> {
	type System = Self;
	fn into_system(self) -> Self::System {
		self
	}
}

impl<Input, F> IntoSystem<Input> for FunctionSystem<Input, F> where 
	FunctionSystem<Input, F>: System
{
	type System = Self;
	fn into_system(self) -> Self::System {
		self
	}
}


mod system_impls {
	#![allow(non_snake_case)]
	#![allow(unused_parens)]
	use super::*;
	impl<F0: System, F1: System> System for Chain<F0, F1> {
		fn run(&mut self, resources: &Resources, entities: &Entities) -> Result<(), ResolveError> {
			if !self.first_finished {
				self.f0.run(resources, entities)?;
				self.first_finished = true;
			}
			self.f1.run(resources, entities)?;
			self.first_finished = false;
			Ok(())
		}
	}

	macro_rules! system_impl {
		($($inputs: ident),*) => {
			// Basic System
			impl <F: FnMut($($inputs),*), $($inputs: SystemInput),*> System for FunctionSystem<($($inputs),*), F> where
				for<'a, 'b> &'a mut F:
					FnMut($(<$inputs as SystemInput>::Item<'b>,)*)
			{
				fn run (&mut self, _resources: &Resources, _entities: &Entities) -> Result<(), ResolveError>{
					fn call_inner<$($inputs),*>(
						mut f: impl FnMut($($inputs),*),
						$($inputs: $inputs),*
					) {
						f($($inputs),*)
					}
					$(let $inputs = $inputs::resolve(_resources, _entities)?;)*
					call_inner(&mut self.f, $($inputs),*);
					Ok(())
				}
			}

			// Convert FnMut into System
			impl<F: FnMut($($inputs),*), $($inputs: SystemInput),*> IntoSystem<($($inputs,)*)> for F where
				for<'a, 'b> &'a mut F:
					FnMut($(<$inputs as SystemInput>::Item<'b>,)*)
			{
				type System = FunctionSystem<($($inputs),*), Self>;
				fn into_system(self) -> Self::System {
					Self::System {
						f: self,
						marker: std::marker::PhantomData,
					}
				}
			}

			// Set
			impl<$($inputs: System),*> System for Set<($(($inputs,bool),)*)> {
				fn run (&mut self, _resources: &Resources, _entities: &Entities) -> Result<(), ResolveError>{
					let ($(ref mut $inputs,)*) = self.set;
					let mut status = Ok(());
					// For each system in set, check if it has already been run before running
					$(
					if $inputs.1 == false {
						if let Err(err) = $inputs.0.run(_resources, _entities) {
							match (&mut status, err) {
								(_, ResolveError::NotFound) => status = Err(ResolveError::NotFound),
								(Ok(()), x) => status = Err(x),
								_ => (),
							}
						} else {
							$inputs.1 = true;
						}
					}
					)*
					// if all systems are run, reset the state
					if status == Ok(()) {
						$($inputs.1 = false;)*
					}
					status
				}
			}
		};
		(;$($inputs: ident $systems: ident),*) => {
			impl<$($systems: IntoSystem<$inputs>,)* $($inputs,)*> IntoSystem<($($inputs,)*)> for ($($systems,)*) {
				type System = Set<($(($systems::System,bool),)*)>;
				fn into_system(self) -> Self::System {
					let ($($systems,)*) = self;
					Set {
						set: ($(($systems.into_system(),false),)*),
					}
				}
			}
		}
	}

	system_impl!();
	system_impl!(I0);
	system_impl!(I0, I1);
	system_impl!(I0, I1, I2);
	system_impl!(I0, I1, I2, I3);
	system_impl!(I0, I1, I2, I3, I4);
	system_impl!(I0, I1, I2, I3, I4, I5);
	system_impl!(I0, I1, I2, I3, I4, I5, I6);
	system_impl!(I0, I1, I2, I3, I4, I5, I6, I7);
	system_impl!(I0, I1, I2, I3, I4, I5, I6, I7, I8);
	system_impl!(I0, I1, I2, I3, I4, I5, I6, I7, I8, I9);
	system_impl!(I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10);
	system_impl!(I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11);
	system_impl!(I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12);
	system_impl!(I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13);
	system_impl!(I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14);
	system_impl!(I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15);
	system_impl!(;);
	system_impl!(;F0 I0);
	system_impl!(;F0 I0, F1 I1);
	system_impl!(;F0 I0, F1 I1, F2 I2);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5, F6 I6);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5, F6 I6, F7 I7);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5, F6 I6, F7 I7, F8 I8);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5, F6 I6, F7 I7, F8 I8, F9 I9);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5, F6 I6, F7 I7, F8 I8, F9 I9, F10 I10);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5, F6 I6, F7 I7, F8 I8, F9 I9, F10 I10, F11 I11);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5, F6 I6, F7 I7, F8 I8, F9 I9, F10 I10, F11 I11, F12 I12);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5, F6 I6, F7 I7, F8 I8, F9 I9, F10 I10, F11 I11, F12 I12, F13 I13);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5, F6 I6, F7 I7, F8 I8, F9 I9, F10 I10, F11 I11, F12 I12, F13 I13, F14 I14);
	system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5, F6 I6, F7 I7, F8 I8, F9 I9, F10 I10, F11 I11, F12 I12, F13 I13, F14 I14, F15 I15);
}

pub use any::*;
mod any {
	use super::*;

	pub trait Resource: Any {}
	impl Resource for i32 {}

	use std::any::{Any, TypeId};
	use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
	use std::collections::HashMap;

	// Resource map
	#[derive(Default)]
	pub struct Resources {
		resources: HashMap<TypeId, RwLock<Box<dyn Any>>>
	}
	
	impl Resources {
		pub fn new () -> Self {
			Self {
				resources: HashMap::new(),
			}
		}

		pub fn add_resource<R: Resource + 'static>(&mut self, resource: R) {
			if !self.resources.insert(resource.type_id(), RwLock::new(Box::new(resource))).is_none() {
				panic!("Attemped to insert resource that already exists");
			}
		}

		pub fn get<'a, R: Resource + 'static>(&'a self) -> Result<RwLockReadGuard<'a, Box<dyn Any>>, ResolveError> {
			if let Some(res) = self.resources.get(&TypeId::of::<R>()) {
				match res.try_read() {
					Ok(res) => Ok(res),
					Err(_) => Err(ResolveError::ReadError)
				}
			} else {
				Err(ResolveError::NotFound)
			}
		}

		pub fn get_mut<'a, R: Resource + 'static>(&'a self) -> Result<RwLockWriteGuard<'a, Box<dyn Any>>, ResolveError> {
			if let Some(res) = self.resources.get(&TypeId::of::<R>()) {
				match res.try_write() {
					Ok(res) => Ok(res),
					Err(_) => Err(ResolveError::WriteError)
				}
			} else {
				Err(ResolveError::NotFound)
			}
		}
	}

	type EntityId = u64;

	#[derive(Default)]
	pub struct Entities {
		// HashMap<TypeId<impl Component>, RwLock<Box<HashMap<EntityId, impl Component>>>
		entities: HashMap<TypeId, RwLock<Box<dyn Any>>>,
		next_id: EntityId,
	}
	impl Entities {
		pub fn new () -> Self {
			Self {
				entities: HashMap::new(),
				next_id: 0,
			}
		}

		pub fn add_entity<E: Entity + 'static>(&mut self, id: EntityId, entity: E) {
		}

		pub(crate) fn add_component_to_entity<C: Component + 'static>(&mut self, id: EntityId, comp: C) {
			if self.get_component_list_mut::<C>().insert(id, comp).is_some() {
				panic!("Entity already has component");
			}
		}

		pub(crate) fn remove_component_from_entity<C: Component + 'static>(&mut self, id: EntityId) {
			if self.get_component_list_mut::<C>().remove(&id).is_none() {
				panic!("Entity does not have component");
			}
		}

		pub(crate) fn get_component_list_mut<C: Component + 'static>(&mut self) -> &mut HashMap<EntityId, C> {
			self.entities.entry(TypeId::of::<HashMap<EntityId, C>>())
				.or_insert(RwLock::new(Box::new(HashMap::<EntityId, C>::new())))
				.get_mut().unwrap().downcast_mut::<HashMap<EntityId, C>>().unwrap()
		}

		pub(crate) fn get_component_list<C: Component + 'static>(&mut self) -> &HashMap<EntityId, C> {
			self.entities.entry(TypeId::of::<HashMap<EntityId, C>>())
				.or_insert(RwLock::new(Box::new(HashMap::<EntityId, C>::new())))
				.get_mut().unwrap().downcast_ref::<HashMap<EntityId, C>>().unwrap()
		}

		pub(super) fn get_new_id(&mut self) -> EntityId {
			self.next_id += 1;
			return self.next_id;
		}
	}
}

pub use entity::*;
mod entity {
	use super::*;

	pub trait Entity {
		fn add_to_entities(self, entities: &mut Entities);
	}
	pub trait Component {}
	macro_rules! impl_entity {
		($($comps: ident),*) => {
			impl<$($comps: Component + 'static),*> Entity for ($($comps),*) {
				fn add_to_entities(self, entities: &mut Entities) {
					let id = entities.get_new_id();
					let ($($comps),*) = self;
					$(entities.add_component_to_entity(id, $comps);)*
				}
			}
		}
	}

	impl_entity!(C0);
	impl_entity!(C0, C1);
	impl_entity!(C0, C1, C2);
}

pub use resource::*;
mod resource {
	use std::sync::{RwLockReadGuard, RwLockWriteGuard};
	use super::*;
	pub struct Res<'a, T: 'a> {
		res: RwLockReadGuard<'a, Box<dyn Any>>,
		marker: std::marker::PhantomData<T>,
	}

	impl<'a, T: 'a + 'static> std::ops::Deref for Res<'a, T> {
		type Target = T;
		fn deref(&self) -> &Self::Target {
			(self.res.deref().deref()).downcast_ref::<T>().unwrap()
		}
	}

	impl<'a, T: 'static + Resource> SystemInput for Res<'a, T> {
		type Item<'new> = Res<'new, T>;
		fn resolve<'r>(resources: &'r Resources, entities: &'r Entities) -> Result<Self::Item<'r>, ResolveError> {
			Ok(Res{res: resources.get::<T>().unwrap(), marker: std::marker::PhantomData})
		}
	}

	pub struct ResMut<'a, T: 'a> {
		res: RwLockWriteGuard<'a, Box<dyn Any>>,
		marker: std::marker::PhantomData<T>,
	}

	impl<'a, T: 'a + 'static> std::ops::Deref for ResMut<'a, T> {
		type Target = T;
		fn deref(&self) -> &Self::Target {
			self.res.deref().deref().downcast_ref::<T>().unwrap()
		}
	}

	impl<'a, T: 'a + 'static> std::ops::DerefMut for ResMut<'a, T> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			self.res.deref_mut().deref_mut().downcast_mut::<T>().unwrap()
		}
	}

	impl<'a, T: 'static + Resource> SystemInput for ResMut<'a, T> {
		type Item<'new> = ResMut<'new, T>;
		fn resolve<'r>(resources: &'r Resources, entities: &'r Entities) -> Result<Self::Item<'r>, ResolveError> {
			Ok(ResMut{res: resources.get_mut::<T>().unwrap(), marker: std::marker::PhantomData})
		}
	}
}
