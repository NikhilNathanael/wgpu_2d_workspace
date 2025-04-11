#[cfg(not(feature="threading"))]
pub trait MaybeSend {}
#[cfg(not(feature="threading"))]
impl<T> MaybeSend for T {}

#[cfg(feature="threading")]
pub trait MaybeSend: Send {}
#[cfg(feature="threading")]
impl<T: Send> MaybeSend for T {}

#[cfg(not(feature="threading"))]
pub trait MaybeSync {}
#[cfg(not(feature="threading"))]
impl<T> MaybeSync for T {}

#[cfg(feature="threading")]
pub trait MaybeSync: Sync {}
#[cfg(feature="threading")]
impl<T: Sync> MaybeSync for T {}
