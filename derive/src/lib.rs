// pub fn add(left: u64, right: u64) -> u64 {
//     left + right
// }

use proc_macro::TokenStream;
use quasiquote::{quasiquote, quote::quote};
use syn::{DataStruct, DeriveInput, Error, Fields, Ident, Index, Type, parse};

#[proc_macro_derive(VertexBufferData)]
pub fn vertex_buffer_data(data: TokenStream) -> TokenStream {
    let strct: DeriveInput = parse(data).unwrap();
    let structname = strct.ident;
    let strct: DataStruct = match strct.data {
        syn::Data::Struct(strct) => strct,
        syn::Data::Enum(x) => {
            return Error::new(
                x.enum_token.span,
                "Vertex Buffer Data cannot be used on enums",
            )
            .to_compile_error()
            .into();
        }
        syn::Data::Union(x) => {
            return Error::new(
                x.union_token.span,
                "Vertex Buffer Data cannot be used on unions",
            )
            .to_compile_error()
            .into();
        }
    };

    let fields: Vec<(Ident, Type)> = {
        match strct.fields {
            Fields::Named(named) => named
                .named
                .into_iter()
                .map(|field| (field.ident.unwrap(), field.ty))
                .collect(),
            Fields::Unnamed(_) => unimplemented!(),
            Fields::Unit => {
                return Error::new(
                    strct.struct_token.span,
                    "Vertex Buffer Data cannot be used on unit structs",
                )
                .to_compile_error()
                .into();
            }
        }
    };

    let wgpu_buffer_path = fields
        .iter()
        .map(|_| quote!(crate::wgpu_context::WGPUBuffer))
        .collect::<Vec<_>>();

    let create_buffers = fields.iter().map(|(_, type_name)|
		quasiquote!(
			crate::wgpu_context::WGPUBuffer::new_vertex((::std::mem::size_of::<#type_name>() * self.len()) as u64, context)
		)
	).collect::<Vec<_>>();

    let fill_buffers = fields.iter().enumerate().map(|(i, (ident, _))|
		quasiquote!(buffers.#{Index::from(i)}.write_iter(self.iter().map(|x| &x.#ident), context))
	).collect::<Vec<_>>();

    let output = quasiquote!(
        impl crate::wgpu_context::BufferData for ::std::vec::Vec<#structname> {
            type Buffers = (#(#wgpu_buffer_path),*);
            fn create_buffers(&self, context: &crate::wgpu_context::WGPUContext) -> Self::Buffers {
                (#(#create_buffers),*)
            }
            fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &crate::wgpu_context::WGPUContext) {
                #(#fill_buffers);*
            }
        }
    );
    return output.into();
}

#[proc_macro_derive(UniformBufferData)]
pub fn uniform_buffer_data(data: TokenStream) -> TokenStream {
    let strct: DeriveInput = parse(data).unwrap();
    let structname = strct.ident;
    match strct.data {
        syn::Data::Struct(_) => (),
        syn::Data::Enum(x) => {
            return Error::new(
                x.enum_token.span,
                "Uniform Buffer Data cannot be used on enums",
            )
            .to_compile_error()
            .into();
        }
        syn::Data::Union(x) => {
            return Error::new(
                x.union_token.span,
                "Uniform Buffer Data cannot be used on unions",
            )
            .to_compile_error()
            .into();
        }
    }

    let output = quote!(
        impl crate::wgpu_context::BufferData for #structname {
            type Buffers = crate::wgpu_context::WGPUBuffer;
            fn create_buffers(&self, context: &crate::wgpu_context::WGPUContext) -> Self::Buffers {
                crate::wgpu_context::WGPUBuffer::new_uniform(::std::mem::size_of::<Self>() as u64, context)
            }
            fn fill_buffers(&self, buffers: &mut Self::Buffers, context: &crate::wgpu_context::WGPUContext) {
                buffers.write_data(::bytemuck::bytes_of(self), context);
            }
        }
    );
    return output.into();
}
