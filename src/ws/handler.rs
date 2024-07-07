use super::InboundMessage;
use std::future::{Future, IntoFuture};

/// Represents a generic event consumer for gateway dispatch events.
pub trait EventConsumer: Send + Sync {
    /// Called when a dispatch event is received.
    async fn handle_event(&mut self, event: InboundMessage);
}

struct FnConsumer<F>(F);

impl<F, Fut: IntoFuture> EventConsumer for FnConsumer<F>
where
    F: Fn(InboundMessage) -> Fut + Send + Sync,
    Fut::IntoFuture: Send,
{
    async fn handle_event(&mut self, event: InboundMessage) {
        (self.0)(event).await;
    }
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
            async fn handle_event(&mut self, event: InboundMessage) {
                use InboundMessage::*;

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
    /// Called when Harmony requests a heartbeat from the client.
    ///
    /// # Note
    /// This heartbeat is automatically sent by the client, this is for debugging purposes only.
    Ping => on_heartbeat_request();

    /// Called when a heartbeat from the client is acknowledged by Harmony.
    Pong => on_heartbeat_ack();

    /// Called when the client is ready to receive events.
    Ready { .. } => on_ready();
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler;

    impl EventHandler for TestHandler {
        async fn on_heartbeat_request(&mut self) {
            println!("Received heartbeat request");
        }
    }

    #[tokio::test]
    async fn test_event_handler() {
        TestHandler.handle_event(InboundMessage::Ping).await;
    }
}