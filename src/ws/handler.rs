use futures_util::future::BoxFuture;
use std::future::{Future, IntoFuture};

use super::Event;
use crate::{models::Message, Context, WithCtx};

/// Represents a generic event consumer for gateway dispatch events.
pub trait EventConsumer: Send + Sync {
    /// Called when a dispatch event is received.
    fn handle_event(&mut self, event: Event) -> impl Future<Output = ()> + Send;
}

struct FnConsumer<F>(F);

impl<F, Fut: IntoFuture> EventConsumer for FnConsumer<F>
where
    F: Fn(Event) -> Fut + Send + Sync,
    Fut::IntoFuture: Send,
{
    async fn handle_event(&mut self, event: Event) {
        (self.0)(event).await;
    }
}

macro_rules! all_the_tuples {
    ($name:ident) => {
        $name!(T1, T2);
        $name!(T1, T2, T3);
        $name!(T1, T2, T3, T4);
        $name!(T1, T2, T3, T4, T5);
        $name!(T1, T2, T3, T4, T5, T6);
        $name!(T1, T2, T3, T4, T5, T6, T7);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
        $name!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
    };
}

macro_rules! impl_compound_handlers {
    ($($t:ident),*) => {
        impl<$($t),*> EventConsumer for ($($t),*)
        where
            $($t: EventConsumer),*
        {
            async fn handle_event(&mut self, event: Event) {
                tokio::join!($($t::handle_event(&mut self.${index()}, event.clone())),*);
            }
        }
    }
}

all_the_tuples!(impl_compound_handlers);

pub(crate) trait EventConsumerErased: Send + Sync {
    fn dyn_handle_event(&mut self, event: Event) -> BoxFuture<'_, ()>;
}

impl<T: EventConsumer> EventConsumerErased for T {
    fn dyn_handle_event(&mut self, event: Event) -> BoxFuture<'_, ()> {
        Box::pin(EventConsumer::handle_event(self, event))
    }
}

/// Creates a raw event consumer from a function.
///
/// # Example
/// ```no_run
/// use adapt::ws::handler;
///
/// let handler = handler::from_fn(|event| async move {
///     println!("Received event: {:?}", event);
/// });
/// ```
///
/// # See Also
/// * [`EventHandler`]: A trait for organizing event handler logic.
/// * [`FallibleEventHandler`]: A trait for organizing event handler logic with error handling.
pub fn from_fn<F, Fut: IntoFuture>(f: F) -> impl EventConsumer
where
    F: Fn(Event) -> Fut + Send + Sync,
    Fut::IntoFuture: Send,
{
    FnConsumer(f)
}

macro_rules! define_event_handlers {
    ($(
        $(#[$doc:meta])*
        $pat:pat => $name:ident($($param:ident: $ty:ty),*);
    )*) => {
        /// Receives and handles events from the gateway such that event handlers can be fallible.
        ///
        /// If you don't need to handle errors, you can use the [`EventHandler`] trait instead.
        pub trait FallibleEventHandler: Send + Sync {
            /// The error type raised by the event handlers.
            type Error: Send;

            /// Called when an error occurs while processing an event within this event handler.
            fn on_error(&mut self, error: Self::Error) -> impl Future<Output = ()> + Send;

            $(
                $(#[$doc])*
                fn $name(
                    &mut self,
                    $($param: $ty),*
                ) -> impl Future<Output = Result<(), Self::Error>> + Send {
                    async {
                        let _ = ($($param),*);
                        Ok(())
                    }
                }
            )*
        }

        impl<E: FallibleEventHandler> EventConsumer for E {
            async fn handle_event(&mut self, event: Event) {
                use Event::*;

                #[allow(unreachable_patterns)]
                if let Err(why) = match event {
                    $($pat => self.$name($($param),*).await,)*
                    _ => Ok(())
                } {
                    self.on_error(why).await;
                }
            }
        }

        /// Receives and handles events from the gateway. Errors must be handled manually within
        /// each event handler.
        ///
        /// # See Also
        /// * [`FallibleEventHandler`]: A fallible version of the [`EventHandler`] trait.
        pub trait EventHandler: Send + Sync {
            $(
                $(#[$doc])*
                fn $name(&mut self, $($param: $ty),*) -> impl Future<Output = ()> + Send {
                    async {
                        let _ = ($($param),*);
                    }
                }
            )*
        }

        impl<E: EventHandler> FallibleEventHandler for E {
            type Error = ();

            async fn on_error(&mut self, _: Self::Error) {}

            $(
                $(#[$doc])*
                async fn $name(&mut self, $($param: $ty),*) -> Result<(), Self::Error> {
                    EventHandler::$name(self, $($param),*).await;
                    Ok(())
                }
            )*
        }
    }
}

define_event_handlers! {
    /// Called when the client is ready to receive events.
    Ready(context) => on_ready(context: Context);

    /// Called when a message is sent.
    MessageCreate(message) => on_message(message: WithCtx<Message>);
}
