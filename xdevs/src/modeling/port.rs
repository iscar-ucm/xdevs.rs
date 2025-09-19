use crate::DynRef;
use std::{any::Any, cell::UnsafeCell, ops::Deref, str::FromStr, sync::Arc};

pub trait PortVal: DynRef + Clone + FromStr + ToString {}

impl<T: DynRef + Clone + FromStr + ToString> PortVal for T {}

/// Trait implemented by DEVS ports. It does not consider message types nor port directions.
pub(crate) trait Port: DynRef {
    /// Port-to-any conversion.
    fn as_any(&self) -> &dyn Any;

    /// Returns `true` if the port does not contain any value.
    ///
    /// # Safety
    ///   
    /// This method must only be executed by an [`InPort`] or
    /// [`super::Component`] when checking if the port is empty.
    unsafe fn is_empty(&self) -> bool;

    /// It clears all the values in the port.
    ///
    /// # Safety
    ///
    /// This method must only be executed by the [`super::Component`] when clearing its ports.
    unsafe fn clear(&self);

    /// Returns `true` if other port is type-compatible.
    fn is_compatible(&self, other: &dyn Port) -> bool;

    /// Propagates messages from the port to other receiving port.
    ///
    /// # Safety
    ///
    /// This method can only be executed by a [`super::Coupled`] model when propagating
    /// messages in its [`crate::simulation::Simulator`] trait implementation.
    unsafe fn propagate(&self, port_to: &dyn Port);

    /// Adds a new value to the port.
    ///
    /// The value is provided as a string and must be parsed to the port's type.
    /// If parsing fails, it returns an error. Otherwise, it returns `Ok(())`.
    #[cfg(feature = "rt")]
    unsafe fn inject(&self, value: &str) -> Result<(), ()>;

    /// Returns a vector of string representations of the values in the port.
    #[cfg(feature = "rt")]
    unsafe fn eject(&self) -> Vec<String>;
}

/// Bag of DEVS messages. Each port has its own bag.
#[derive(Debug)]
pub(super) struct Bag<T>(UnsafeCell<Vec<T>>);

impl<T> Bag<T> {
    /// Creates a new message bag wrapped in an [`Arc`].
    #[inline]
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self(UnsafeCell::new(Vec::new())))
    }

    /// Returns a reference to the vector of messages in the bag.
    ///
    /// # Safety:
    ///
    /// The caller must ensure that it fulfills **any** of the following invariants:
    ///
    /// - The caller is an [`InPort`] struct and fulfills the aditional invariants.
    /// - The caller executed the [`Port::propagate`] method and fulfills the additional invariants.
    #[inline]
    unsafe fn borrow(&self) -> &Vec<T> {
        &*self.get()
    }

    /// Returns a mutable reference to the vector of messages in the bag.
    ///
    /// # Safety:
    ///
    /// The caller must ensure that it fulfills **any** of the following invariants:
    ///
    /// - The caller is an [`OutPort`] struct and fulfills the aditional invariants.
    /// - The caller executed the [`Port::propagate`] method and fulfills the additional invariants.
    #[allow(clippy::mut_from_ref)]
    #[inline]
    unsafe fn borrow_mut(&self) -> &mut Vec<T> {
        &mut *self.get()
    }
}

impl<T> Deref for Bag<T> {
    type Target = UnsafeCell<Vec<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(any(feature = "par_any", feature = "rt"))]
// Safety: if all the invariants are met, then a bag can be safely shared among threads.
unsafe impl<T: Send> Send for Bag<T> {}

#[cfg(any(feature = "par_any", feature = "rt"))]
// Safety: if all the invariants are met, then a bag can be safely shared among threads.
unsafe impl<T: Sync> Sync for Bag<T> {}

impl<T: PortVal> Port for Bag<T> {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    unsafe fn is_empty(&self) -> bool {
        self.borrow().is_empty()
    }

    #[inline]
    unsafe fn clear(&self) {
        self.borrow_mut().clear();
    }

    #[inline]
    fn is_compatible(&self, other: &dyn Port) -> bool {
        other.as_any().downcast_ref::<Bag<T>>().is_some()
    }

    #[inline]
    unsafe fn propagate(&self, port_to: &dyn Port) {
        let port_to = port_to.as_any().downcast_ref::<Bag<T>>().unwrap();
        port_to.borrow_mut().extend_from_slice(self.borrow());
    }

    #[inline]
    #[cfg(feature = "rt")]
    unsafe fn inject(&self, value: &str) -> Result<(), ()> {
        match value.parse() {
            Ok(value) => {
                self.borrow_mut().push(value);
                Ok(())
            }
            Err(_) => Err(()),
        }
    }

    #[inline]
    #[cfg(feature = "rt")]
    unsafe fn eject(&self) -> Vec<String> {
        self.borrow_mut().iter().map(|v| v.to_string()).collect()
    }
}

/// Input port. This structure only allows reading messages. Thus, it cannot inject messages.
///
/// Note that we do not implement the [`Clone`] trait in purpose, as we want to avoid their misuse.
#[derive(Debug)]
pub struct InPort<T>(pub(super) Arc<Bag<T>>);

impl<T> InPort<T> {
    /// Returns `true` if the underlying bag is empty. Otherwise, it returns `false`.
    ///
    /// # Safety
    ///
    /// This method can only be called within the [`Atomic::delta_ext`](super::Atomic::delta_ext) method.
    /// Furthermore, this port must be one of the input ports of the implementer.
    #[inline]
    pub unsafe fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    /// Returns a reference to the slice of messages of the underlying bag.
    ///
    /// # Safety
    ///
    /// This method can only be called when implementing the [`super::Atomic::delta_ext`] method.
    /// Furthermore, this port must be one of the input ports of the implementer.
    #[inline]
    pub unsafe fn get_values(&self) -> &[T] {
        self.0.borrow()
    }
}

/// Output port. This structure can only inject messages. Thus, it cannot read messages.
///
/// Note that we do not implement the [`Clone`] trait in purpose, as we want to avoid their misuse.
#[derive(Debug)]
pub struct OutPort<T>(pub(super) Arc<Bag<T>>);

impl<T> OutPort<T> {
    /// Adds a new value to the output port.
    ///
    /// # Safety
    ///
    /// This method can only be called when implementing the [`Atomic::lambda`](super::Atomic::lambda) method.
    /// Furthermore, this port must be one of the output ports of the implementer.
    #[inline]
    pub unsafe fn add_value(&self, value: T) {
        self.0.borrow_mut().push(value);
    }

    /// Adds new values from a slice to the output port.
    ///
    /// # Safety
    ///
    /// This method can only be called when implementing the [`Atomic::lambda`](super::Atomic::lambda) method.
    /// Furthermore, this port must be one of the output ports of the implementer.
    #[inline]
    pub unsafe fn add_values(&self, values: &[T])
    where
        T: Clone,
    {
        self.0.borrow_mut().extend_from_slice(values);
    }
}
