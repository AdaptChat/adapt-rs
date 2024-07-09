use super::InboundMessage;
use crate::models::Message;
use crate::{Context, WithCtx};

/// Represents a resolved dispatch event received from the gateway.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum Event {
    /// The client is ready to receive events.
    Ready(Context),
    /// A resolvable message was sent.
    MessageCreate(WithCtx<Message>),
}

pub fn populate(ctx: Context, event: InboundMessage, pending: &mut Vec<Event>) {
    match event {
        InboundMessage::Ready { .. } => pending.push(Event::Ready(ctx)),
        InboundMessage::MessageCreate { message, .. } => {
            pending.push(Event::MessageCreate(ctx.with(Message::from_raw(message))));
        }
        _ => (),
    }
}
