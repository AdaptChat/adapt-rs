use crate::http::endpoints;
use crate::models::channel::ChannelId;
use crate::{Context, Result, WithCtx};

use essence::http::message::CreateMessagePayload;
use std::ops::Deref;

crate::id_type! {
    /// Represents an Adapt message by its ID.
    ///
    /// # Note
    /// Most endpoints that require a message ID also require a channel ID. This type is not aware
    /// of the channel ID, thus all message functionality is in [`PartialMessage`] instead.
    pub struct MessageId: Message;
}

/// Represents anything that can be converted into a [`CreateMessagePayload`].
pub trait IntoCreateMessage {
    /// Converts the implementor into a message payload.
    fn into_create_message(self) -> CreateMessagePayload;
}

impl IntoCreateMessage for CreateMessagePayload {
    fn into_create_message(self) -> CreateMessagePayload {
        self
    }
}

impl IntoCreateMessage for String {
    fn into_create_message(self) -> CreateMessagePayload {
        CreateMessagePayload {
            content: Some(self),
            ..Default::default()
        }
    }
}

impl IntoCreateMessage for &str {
    fn into_create_message(self) -> CreateMessagePayload {
        CreateMessagePayload {
            content: Some(self.to_string()),
            ..Default::default()
        }
    }
}

/// Represents an Adapt message by its ID, aware of its parent channel ID.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[must_use = "this struct does nothing on its own"]
pub struct PartialMessage {
    /// The ID of the message.
    pub id: MessageId,
    /// The ID of the channel the message belongs to.
    pub channel_id: ChannelId,
}

impl PartialMessage {
    /// Creates a new partial message from a channel ID and message ID.
    pub const fn new(channel_id: ChannelId, message_id: MessageId) -> Self {
        Self {
            id: message_id,
            channel_id,
        }
    }

    /// Adds context to the message, allowing it to access shared client state.
    pub const fn with_ctx(self, ctx: Context) -> WithCtx<Self> {
        ctx.with(self)
    }
}

impl WithCtx<PartialMessage> {
    /// Deletes the message.
    pub async fn delete(&self) -> Result<()> {
        self.ctx
            .http()
            .request(endpoints::DeleteMessage(*self.channel_id, *self.id))
            .await
    }
}

/// Represents an Adapt message.
#[derive(Clone, Debug)]
pub struct Message {
    /// The underlying partial message.
    partial: PartialMessage,
    /// The text content of the message. This is an empty string if the message has no content.
    pub content: String,
}

impl Message {
    /// Creates a new message from a raw [`essence::models::Message`].
    #[must_use]
    pub fn from_raw(message: essence::models::Message) -> Self {
        Self {
            partial: PartialMessage::new(message.channel_id.into(), message.id.into()),
            content: message.content.unwrap_or_default(),
        }
    }

    /// Creates a copyable [`PartialMessage`] from this message.
    pub const fn partial(&self) -> PartialMessage {
        self.partial
    }

    /// Returns the ID of the message.
    #[must_use]
    pub const fn id(&self) -> MessageId {
        self.partial.id
    }

    /// Returns the ID of the channel the message belongs to.
    #[must_use]
    pub const fn channel_id(&self) -> ChannelId {
        self.partial.channel_id
    }
}

impl WithCtx<Message> {
    /// Creates a copyable [`PartialMessage`] from this message.
    pub fn partial(&self) -> WithCtx<PartialMessage> {
        self.ctx.clone().with(self.inner().partial())
    }

    /// Returns the ID of the message.
    pub fn id(&self) -> WithCtx<MessageId> {
        self.ctx.clone().with(self.inner().id())
    }

    /// Returns the ID of the channel the message belongs to.
    pub fn channel_id(&self) -> WithCtx<ChannelId> {
        self.ctx.clone().with(self.inner().channel_id())
    }
}

impl Deref for Message {
    type Target = PartialMessage;

    fn deref(&self) -> &Self::Target {
        &self.partial
    }
}

crate::impl_common_traits!(Message);
