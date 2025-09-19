pub mod devstone;
pub mod gpt;
pub mod modeling;
pub mod simulation;

/// Helper trait for avoiding verbose trait constraints.
#[cfg(not(any(feature = "par_any", feature = "rt")))]
pub trait DynRef: 'static {}
/// Helper trait for avoiding verbose trait constraints.
#[cfg(any(feature = "par_any", feature = "rt"))]
pub trait DynRef: 'static + Sync + Send {}

#[cfg(not(any(feature = "par_any", feature = "rt")))]
impl<T: 'static + ?Sized> DynRef for T {}
#[cfg(any(feature = "par_any", feature = "rt"))]
impl<T: 'static + Sync + Send + ?Sized> DynRef for T {}

/// In xDEVS, events are defined as tuples port-value.
/// The `Event` struct provides a string representation of an event.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Event(String, String);

impl Event {
    /// Creates a new `Event` tuple from types that can be converted into `String`s.
    pub fn new<S: Into<String>, T: Into<String>>(port: S, value: T) -> Self {
        Self(port.into(), value.into())
    }

    /// Returns the name of the port of the `Event`.
    #[inline]
    pub fn port(&self) -> &str {
        &self.0
    }

    /// Returns the a string representation of the value of the `Event`.
    #[inline]
    pub fn value(&self) -> &str {
        &self.1
    }
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.port(), self.value())
    }
}
