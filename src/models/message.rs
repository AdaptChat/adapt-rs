crate::id_type! {
    /// Represents an Adapt message by its ID.
    pub struct MessageId: Message;
}

/// Represents an Adapt message.
#[derive(Clone, Debug)]
pub struct Message {
    /// The ID of the message.
    pub id: MessageId,
}

impl Message {
    /// Creates a new message from a raw [`essence::models::Message`].
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_raw(message: essence::models::Message) -> Self {
        Self {
            id: message.id.into(),
        }
    }
}

impl std::ops::Deref for Message {
    type Target = MessageId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}
