use std::any::{TypeId, Any};
use std::collections::HashMap;

struct Scheduler {
	schedule: HashMap<Schedule, Vec<Box<dyn System>>>,
	resources: AnyMap,
}

pub enum Schedule {
	Startup,
	Update,
	Render,
}

pub struct Chain<F0, F1> {
	f0: F0,
	f1: F1,
}

impl<F0: System, F1: System> System for Chain<F0, F1> {
	fn run(&mut self, resources: &AnyMap, entities: &AnyMap) {
		self.f0.run(resources, entities);
		self.f1.run(resources, entities);
	}
}

pub struct Set<F> {
	set: F,
}

pub struct FunctionSystem<Input, F> {
	f: F,
	marker: std::marker::PhantomData<fn() -> Input>,
}

pub type AnyMap = HashMap<TypeId, Box<dyn Any>>;

pub trait SystemInput {
	fn resolve(resources: &AnyMap, entities: &AnyMap) -> Self;
}

pub trait System {
	fn run(&mut self, resources: &AnyMap, entities: &AnyMap);
}

pub trait IntoSystem<Input> {
	type System: System;
	fn into_system(self) -> Self::System;
	fn chain<Other: IntoSystem<I1>, I1>(self, other: Other) -> Chain<Self::System, Other::System> where Self: Sized {
		Chain {
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

macro_rules! system_impl {
	($($inputs: ident),*) => {
		// Basic System
		impl <F: FnMut($($inputs),*), $($inputs: SystemInput),*> System for FunctionSystem<($($inputs),*), F> {
			fn run (&mut self, resources: &AnyMap, entities: &AnyMap) {
				$(let $inputs = SystemInput::resolve(resources, entities);)*
				(self.f)($($inputs),*);
			}
		}

		// Convert FnMut into System
		impl<F: FnMut($($inputs),*), $($inputs: SystemInput),*> IntoSystem<($($inputs,)*)> for F {
			type System = FunctionSystem<($($inputs),*), Self>;
			fn into_system(self) -> Self::System {
				Self::System {
					f: self,
					marker: std::marker::PhantomData,
				}
			}
		}

		// Set
		impl<$($inputs: System),*> System for Set<($($inputs,)*)> {
			fn run (&mut self, resources: &AnyMap, entities: &AnyMap) {
				let ($(ref mut $inputs,)*) = self.set;
				$($inputs.run(resources, entities);)*
			}
		}

		// Convert Tuple of FnMut into Set 
		
		// Tuple of systems is a set of systems that may run at the same time
		// I Dont this this is a good idea

		// impl<$($inputs: System),*> System for ($($inputs),*) {
		// 	fn run(&mut self, resources: &AnyMap, entities: &AnyMap) {
		// 		let ($(ref mut $inputs,)*) = self;
		// 		$($inputs.run(resources, entities);)*
		// 	}
		// }
	};
	(;$($inputs: ident $systems: ident),*) => {
		impl<$($systems: IntoSystem<$inputs>,)* $($inputs,)*> IntoSystem<($($inputs,)*)> for ($($systems,)*) {
			type System = Set<($($systems::System,)*)>;
			fn into_system(self) -> Self::System {
				let ($($systems,)*) = self;
				Set {
					set: ($($systems.into_system(),)*),
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
system_impl!(I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15, I16);
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
system_impl!(;F0 I0, F1 I1, F2 I2, F3 I3, F4 I4, F5 I5, F6 I6, F7 I7, F8 I8, F9 I9, F10 I10, F11 I11, F12 I12, F13 I13, F14 I14, F15 I15, F16 I16);
