use crate::http::endpoints;
use crate::models::message::IntoCreateMessage;
use crate::models::{Id, Message, MessageId, PartialMessage};
use crate::{Context, Result, WithCtx};

crate::id_type! {
    /// Represents an Adapt channel by its ID.
    pub struct ChannelId: Channel;
}

impl ChannelId {
    /// Gets a [`PartialMessage`] in this channel by its message ID.
    pub const fn partial_message(&self, message_id: MessageId) -> PartialMessage {
        PartialMessage::new(*self, message_id)
    }

    /// Attaches a [`Context`] to this channel ID to allow it to access shared client state.
    pub const fn with_ctx(self, ctx: Context) -> WithCtx<Self> {
        ctx.with(self)
    }
}

impl WithCtx<ChannelId> {
    /// Gets a [`PartialMessage`] in this channel by its message ID.
    pub fn partial_message(&self, message_id: MessageId) -> WithCtx<PartialMessage> {
        self.ctx
            .clone()
            .with(self.inner().partial_message(message_id))
    }

    /// Creates a new message in this channel.
    pub async fn send(&self, payload: impl IntoCreateMessage + Send) -> Result<WithCtx<Message>> {
        let message = self
            .ctx
            .http()
            .request(endpoints::CreateMessage(self.get()))
            .body(payload.into_create_message())
            .await?;

        Ok(self.ctx.clone().with(Message::from_raw(message)))
    }
}
