//! Properties: a constant, a signal, a memo, or a closure (specs/05, "Properties").

use quadra_reactive::{Memo, Signal};

/// A property as given to a view. A constant is written at mount and costs nothing after;
/// only a reactive property makes a binding.
pub enum Prop<T> {
    Value(T),
    Reactive(Box<dyn Fn() -> T>),
}

/// What a property setter accepts. The marker keeps the implementations apart: a value, a
/// closure and a signal are different ways into the same property.
pub trait IntoProp<T, Marker> {
    fn into_prop(self) -> Prop<T>;
}

pub struct ByValue;
pub struct ByClosure;
pub struct BySignal;
pub struct ByMemo;

impl<T, U: Into<T>> IntoProp<T, ByValue> for U {
    fn into_prop(self) -> Prop<T> {
        Prop::Value(self.into())
    }
}

impl<T, U: Into<T>, F: Fn() -> U + 'static> IntoProp<T, ByClosure> for F {
    fn into_prop(self) -> Prop<T> {
        Prop::Reactive(Box::new(move || self().into()))
    }
}

impl<T, U: Into<T> + Clone + 'static> IntoProp<T, BySignal> for Signal<U> {
    fn into_prop(self) -> Prop<T> {
        Prop::Reactive(Box::new(move || self.get().into()))
    }
}

impl<T, U: Into<T> + Clone + 'static> IntoProp<T, ByMemo> for Memo<U> {
    fn into_prop(self) -> Prop<T> {
        Prop::Reactive(Box::new(move || self.get().into()))
    }
}
