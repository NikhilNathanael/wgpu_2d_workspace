mod vector {
	use bytemuck::{Pod, Zeroable};
	use std::ops::{Deref, DerefMut};

	// Definition of Vector types
	macro_rules! impl_def {
		($outer_name: tt, $actual_len: literal, $deref_len: literal) => {
			#[derive(Debug, Clone, Copy, Pod, Zeroable)]
			#[repr(transparent)]
			pub struct $outer_name<T> {
				data: [T;$actual_len]
			}

			impl<T: Zeroable> $outer_name<T> {
				pub fn new (data: [T;$deref_len]) -> Self {
					let mut output = <[T;$actual_len]>::zeroed();
					unsafe{(&mut output as *mut [T;$actual_len] as *mut [T;$deref_len]).write(data)};
					Self {
						data: output,
					}
				}
			}

			impl<T: Copy + Zeroable> From<T> for $outer_name<T> {
				fn from (data: T) -> Self {
					Self::new([data;$deref_len])
				}
			}

			impl<T> Deref for $outer_name<T> {
				type Target = [T;$deref_len];
				fn deref(&self) -> &Self::Target {
					debug_assert!($deref_len <= $actual_len);
					eprintln!("{:?} {:?}", $deref_len, $actual_len);
					unsafe{&*(self.data.as_ptr().cast())}
				}
			}

			impl<T> DerefMut for $outer_name<T> {
				fn deref_mut(&mut self) -> &mut Self::Target {
					debug_assert!($deref_len <= $actual_len);
					unsafe{&mut *(self.data.as_mut_ptr().cast())}
				}
			}
		}
	}

	// Used in dot product implementation
	macro_rules! strip_plus {
		(+ $rest: expr) => {$rest};
	}
	
	// All add, sub, mul, and div implementations
	macro_rules! impl_math {
		($vector_ty: ty, $inner_ty: ty, $($normal_indeces: literal),* $(; $($default_indeces: literal),*)?) => {
			impl $vector_ty {
				pub fn dot (&self, other: Self) -> $inner_ty {
					strip_plus!($(+ self.data[$normal_indeces] * other.data[$normal_indeces])+)
				}
			}

			// Vector x Scalar
				impl<'a> Add<&'a $inner_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn add(self, other: &'a $inner_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self.data[$normal_indeces] + other),+,
								$($(self.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Add<&'a $inner_ty> for $vector_ty {
					type Output = Self;
					fn add (self, other: &'a $inner_ty) -> Self {
						&self + other
					}
				}

				impl<'a> Add<$inner_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn add (self, other: $inner_ty) -> $vector_ty {
						self + &other
					}
				}

				impl Add<$inner_ty> for $vector_ty {
					type Output = Self;
					fn add (self, other: $inner_ty) -> Self {
						&self + &other
					}
				}

				impl<'a> Sub<&'a $inner_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn sub(self, other: &'a $inner_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self.data[$normal_indeces] - other),+,
								$($(self.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Sub<&'a $inner_ty> for $vector_ty {
					type Output = Self;
					fn sub (self, other: &'a $inner_ty) -> Self {
						&self - other
					}
				}

				impl<'a> Sub<$inner_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn sub (self, other: $inner_ty) -> $vector_ty {
						self - &other
					}
				}

				impl Sub<$inner_ty> for $vector_ty {
					type Output = Self;
					fn sub (self, other: $inner_ty) -> Self {
						&self - &other
					}
				}

				impl<'a> Mul<&'a $inner_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn mul(self, other: &'a $inner_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self.data[$normal_indeces] * other),+,
								$($(self.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Mul<&'a $inner_ty> for $vector_ty {
					type Output = Self;
					fn mul (self, other: &'a $inner_ty) -> Self {
						&self * other
					}
				}

				impl<'a> Mul<$inner_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn mul (self, other: $inner_ty) -> $vector_ty {
						self * &other
					}
				}

				impl Mul<$inner_ty> for $vector_ty {
					type Output = Self;
					fn mul (self, other: $inner_ty) -> Self {
						&self * &other
					}
				}

				impl<'a> Div<&'a $inner_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn div(self, other: &'a $inner_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self.data[$normal_indeces] / other),+,
								$($(self.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Div<&'a $inner_ty> for $vector_ty {
					type Output = Self;
					fn div (self, other: &'a $inner_ty) -> Self {
						&self / other
					}
				}

				impl<'a> Div<$inner_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn div (self, other: $inner_ty) -> $vector_ty {
						self / &other
					}
				}

				impl Div<$inner_ty> for $vector_ty {
					type Output = Self;
					fn div (self, other: $inner_ty) -> Self {
						&self / &other
					}
				}

			// Scalar x Vector
				impl<'a> Add<&'a $vector_ty> for &'a $inner_ty {
					type Output = $vector_ty;
					fn add(self, other: &'a $vector_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self + other.data[$normal_indeces]),+,
								$($(other.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Add<&'a $vector_ty> for $inner_ty {
					type Output = $vector_ty;
					fn add (self, other: &'a $vector_ty) -> Self::Output {
						&self + other
					}
				}

				impl<'a> Add<$vector_ty> for &'a $inner_ty {
					type Output = $vector_ty;
					fn add (self, other: $vector_ty) -> $vector_ty {
						self + &other
					}
				}

				impl Add<$vector_ty> for $inner_ty {
					type Output = $vector_ty;
					fn add (self, other: $vector_ty) -> Self::Output {
						&self + &other
					}
				}

				impl<'a> Sub<&'a $vector_ty> for &'a $inner_ty {
					type Output = $vector_ty;
					fn sub(self, other: &'a $vector_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self - other.data[$normal_indeces]),+,
								$($(other.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Sub<&'a $vector_ty> for $inner_ty {
					type Output = $vector_ty;
					fn sub (self, other: &'a $vector_ty) -> Self::Output {
						&self - other
					}
				}

				impl<'a> Sub<$vector_ty> for &'a $inner_ty {
					type Output = $vector_ty;
					fn sub (self, other: $vector_ty) -> $vector_ty {
						self - &other
					}
				}

				impl Sub<$vector_ty> for $inner_ty {
					type Output = $vector_ty;
					fn sub (self, other: $vector_ty) -> Self::Output {
						&self - &other
					}
				}

				impl<'a> Mul<&'a $vector_ty> for &'a $inner_ty {
					type Output = $vector_ty;
					fn mul(self, other: &'a $vector_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self * other.data[$normal_indeces]),+,
								$($(other.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Mul<&'a $vector_ty> for $inner_ty {
					type Output = $vector_ty;
					fn mul (self, other: &'a $vector_ty) -> Self::Output {
						&self * other
					}
				}

				impl<'a> Mul<$vector_ty> for &'a $inner_ty {
					type Output = $vector_ty;
					fn mul (self, other: $vector_ty) -> $vector_ty {
						self * &other
					}
				}

				impl Mul<$vector_ty> for $inner_ty {
					type Output = $vector_ty;
					fn mul (self, other: $vector_ty) -> Self::Output {
						&self * &other
					}
				}

				impl<'a> Div<&'a $vector_ty> for &'a $inner_ty {
					type Output = $vector_ty;
					fn div(self, other: &'a $vector_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self / other.data[$normal_indeces]),+,
								$($(other.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Div<&'a $vector_ty> for $inner_ty {
					type Output = $vector_ty;
					fn div (self, other: &'a $vector_ty) -> Self::Output {
						&self / other
					}
				}

				impl<'a> Div<$vector_ty> for &'a $inner_ty {
					type Output = $vector_ty;
					fn div (self, other: $vector_ty) -> $vector_ty {
						self / &other
					}
				}

				impl Div<$vector_ty> for $inner_ty {
					type Output = $vector_ty;
					fn div (self, other: $vector_ty) -> Self::Output {
						&self / &other
					}
				}

			// Vector x Vector
				impl<'a> Add<&'a $vector_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn add(self, other: &'a $vector_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self.data[$normal_indeces] + other.data[$normal_indeces]),+,
								$($(self.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Add<&'a $vector_ty> for $vector_ty {
					type Output = Self;
					fn add (self, other: &'a $vector_ty) -> Self {
						&self + other
					}
				}

				impl<'a> Add<$vector_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn add (self, other: $vector_ty) -> $vector_ty {
						self + &other
					}
				}

				impl Add<$vector_ty> for $vector_ty {
					type Output = Self;
					fn add (self, other: Self) -> Self {
						&self + &other
					}
				}

				impl<'a> Sub<&'a $vector_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn sub(self, other: &'a $vector_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self.data[$normal_indeces] - other.data[$normal_indeces]),+,
								$($(self.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Sub<&'a $vector_ty> for $vector_ty {
					type Output = Self;
					fn sub (self, other: &'a $vector_ty) -> Self {
						&self - other
					}
				}

				impl<'a> Sub<$vector_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn sub (self, other: $vector_ty) -> $vector_ty {
						self - &other
					}
				}

				impl Sub<$vector_ty> for $vector_ty {
					type Output = Self;
					fn sub (self, other: Self) -> Self {
						&self - &other
					}
				}

				impl<'a> Mul<&'a $vector_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn mul(self, other: &'a $vector_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self.data[$normal_indeces] * other.data[$normal_indeces]),+,
								$($(self.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Mul<&'a $vector_ty> for $vector_ty {
					type Output = Self;
					fn mul (self, other: &'a $vector_ty) -> Self {
						&self * other
					}
				}

				impl<'a> Mul<$vector_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn mul (self, other: $vector_ty) -> $vector_ty {
						self * &other
					}
				}

				impl Mul<$vector_ty> for $vector_ty {
					type Output = Self;
					fn mul (self, other: Self) -> Self {
						&self * &other
					}
				}

				impl<'a> Div<&'a $vector_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn div(self, other: &'a $vector_ty) -> Self::Output {
						Self::Output {
							data: [
								$(self.data[$normal_indeces] / other.data[$normal_indeces]),+,
								$($(self.data[$default_indeces]),*)?
							]
						}
					}
				}

				impl<'a> Div<&'a $vector_ty> for $vector_ty {
					type Output = Self;
					fn div (self, other: &'a $vector_ty) -> Self {
						&self / other
					}
				}

				impl<'a> Div<$vector_ty> for &'a $vector_ty {
					type Output = $vector_ty;
					fn div (self, other: $vector_ty) -> $vector_ty {
						self / &other
					}
				}

				impl Div<$vector_ty> for $vector_ty {
					type Output = Self;
					fn div (self, other: Self) -> Self {
						&self / &other
					}
				}
		}
	}

	// tests for the above implemenations
	macro_rules! impl_math_tests {
		($inner_ty: ty, $outer_ty: tt, $size: literal, $($indeces: literal),+) => {
			#[cfg(test)]
			#[test]
			fn add_test () {
				let mut rng = rng();
				(0..200).for_each(|_| {
					let x: [$inner_ty;$size] = rng.random();
					let y: [$inner_ty;$size] = rng.random();

					let sum_normal = [$(x[$indeces] + y[$indeces]),+];

					let z = $outer_ty::<$inner_ty>::new(x) + $outer_ty::<$inner_ty>::new(y);
					assert_eq!(&sum_normal, z.deref());
				});
			}

			#[cfg(test)]
			#[test]
			fn sub_test () {
				let mut rng = rng();
				(0..200).for_each(|_| {
					let x: [$inner_ty;$size] = rng.random();
					let y: [$inner_ty;$size] = rng.random();

					let diff_normal = [$(x[$indeces] - y[$indeces]),+];

					let z = $outer_ty::<$inner_ty>::new(x) - $outer_ty::<$inner_ty>::new(y);
					assert_eq!(&diff_normal, z.deref());
				});
			}

			#[cfg(test)]
			#[test]
			fn mul_test () {
				let mut rng = rng();
				(0..200).for_each(|_| {
					let x: [$inner_ty;$size] = rng.random();
					let y: [$inner_ty;$size] = rng.random();

					let mul_normal = [$(x[$indeces] * y[$indeces]),+];

					let z = $outer_ty::<$inner_ty>::new(x) * $outer_ty::<$inner_ty>::new(y);
					assert_eq!(&mul_normal, z.deref());
				});
			}

			#[cfg(test)]
			#[test]
			fn div_test () {
				let mut rng = rng();
				(0..200).for_each(|_| {
					let x: [$inner_ty;$size] = rng.random();
					let y: [$inner_ty;$size] = rng.random();

					let div_normal = [$(x[$indeces] / y[$indeces]),+];

					let z = $outer_ty::<$inner_ty>::new(x) / $outer_ty::<$inner_ty>::new(y);
					assert_eq!(&div_normal, z.deref());
				});
			}

			#[cfg(test)]
			#[test]
			fn validity_test() {
				let mut rng = rng();
				println!("{:?}", $outer_ty::<$inner_ty>::new(rng.random()).deref());
			}

			#[cfg(test)]
			#[test]
			fn dot_test() {
				let mut rng = rng();
				(0..200).for_each(|_| {
					let x: [$inner_ty;$size] = rng.random();
					let y: [$inner_ty;$size] = rng.random();

					let dot_normal = strip_plus!($(+ x[$indeces] * y[$indeces])+);

					let z = $outer_ty::<$inner_ty>::new(x).dot($outer_ty::<$inner_ty>::new(y));
					assert_eq!(dot_normal, z);
				});
			}
		}
	}

	// 2D Rotations 
	impl Vector2<f32> {
		pub fn rotation(angle: f32) -> Self {
			Self {
				data: [
					angle.cos(),
					angle.sin(),
				]
			}
		}
	}

	// Cross Product is only available in 3 dimensions
	impl Vector3<f32> {
		pub fn cross_product (&self, other: &Self) -> Self {
			Self {
				data: [
					self.data[1] * other.data[2] - self.data[2] * other.data[1],
					- self.data[0] * other.data[2] + self.data[2] * other.data[0],
					self.data[0] * other.data[1] - self.data[1] * other.data[0],
					0.
				]
			}
		}
	}

	impl Vector3<i32> {
		pub fn cross_product (&self, other: &Self) -> Self {
			Self {
				data: [
					self.data[1] * other.data[2] - self.data[2] * other.data[1],
					- self.data[0] * other.data[2] + self.data[2] * other.data[0],
					self.data[0] * other.data[1] - self.data[1] * other.data[0],
					0
				]
			}
		}
	}

	impl_def!(Vector2, 2, 2);
	impl_def!(Vector3, 4, 3);
	impl_def!(Vector4, 4, 4);

	use std::ops::{Add, Sub, Mul, Div};
	impl_math!(Vector2<f32>, f32, 0, 1);
	impl_math!(Vector2<i32>, i32, 0, 1);

	impl_math!(Vector3<f32>, f32, 0, 1, 2, 3);
	impl_math!(Vector3<i32>, i32, 0, 1, 2; 3);

	impl_math!(Vector4<f32>, f32, 0, 1, 2, 3);
	impl_math!(Vector4<i32>, i32, 0, 1, 2, 3);

	mod vector2_f32_tests{
		impl_math_tests!(f32, Vector2, 2, 0, 1);
	}
	mod vector3_f32_tests{
		impl_math_tests!(f32, Vector3, 3, 0, 1, 2);
	}
	mod vector4_f32_tests{
		impl_math_tests!(f32, Vector4, 4, 0, 1, 2, 3);
	}
}

pub use vector::*;
