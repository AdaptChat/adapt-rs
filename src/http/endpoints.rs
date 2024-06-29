#![allow(unused_parens)]
#![allow(clippy::wildcard_imports)]

use essence::{http::*, models};
use serde::{Deserialize, Serialize};

macro_rules! endpoints {
    ($(
        $(#[$doc:meta])* $name:ident $(<$($lt:lifetime),+>)? $(($($params:ident: $ty:ty),+))?
        $(query($query:ty))? $(body($body:ty))? $(resp($resp:ty))? = $method:ident $path:literal;
    )+) => {
        $(
            $(#[$doc])*
            #[derive(Copy, Clone, Debug, PartialEq, Eq)]
            pub struct $name $(<$($lt),+>)? $(( $(pub $ty),+ ))?;

            impl $(<$($lt),+>)? $name $(<$($lt),+>)? {
                $($(
                    #[inline]
                    #[doc = concat!("Returns the `", stringify!($params), "` parameter of the endpoint.")]
                    const fn $params(&self) -> $ty {
                        self.${index()}
                    }
                )+)?
            }

            impl $(<$($lt),+>)? Endpoint for $name $(<$($lt),+>)? {
                const METHOD: reqwest::Method = reqwest::Method::$method;
                const PATH: &'static str = $path;

                type Query = ($($query)?);
                type Body = ($($body)?);
                type Response = ($($resp)?);

                #[inline]
                fn path(&self) -> String {
                    format!($path, $($($params = self.$params()),+)?)
                }
            }
        )+
    }
}

endpoints! {
    // Channels
    GetChannel(channel_id: u64) resp(models::Channel) = GET "/channels/{channel_id}";
    EditChannel(channel_id: u64)
        body(channel::EditChannelPayload) resp(models::Channel) = PATCH "/channels/{channel_id}";
    DeleteChannel(channel_id: u64) = DELETE "/channels/{channel_id}";
    GetGuildChannels(guild_id: u64) resp(Vec<models::Channel>) = GET "/guilds/{guild_id}/channels";
    CreateGuildChannel(guild_id: u64)
        body(channel::CreateGuildChannelPayload) resp(models::Channel) = POST "/guilds/{guild_id}/channels";

    // Messages
    GetMessageHistory(channel_id: u64)
        query(message::MessageHistoryQuery) resp(Vec<models::Message>) = GET "/channels/{channel_id}/messages";
    CreateMessage(channel_id: u64)
        body(message::CreateMessagePayload) resp(models::Message) = POST "/channels/{channel_id}/messages";
    GetMessage(channel_id: u64, message_id: u64)
        resp(models::Message) = GET "/channels/{channel_id}/messages/{message_id}";
    EditMessage(channel_id: u64, message_id: u64)
        body(message::EditMessagePayload) resp(models::Message) = PATCH "/channels/{channel_id}/messages/{message_id}";
    DeleteMessage(channel_id: u64, message_id: u64) = DELETE "/channels/{channel_id}/messages/{message_id}";
    PinMessage(channel_id: u64, message_id: u64) = PUT "/channels/{channel_id}/messages/{message_id}/pin";
    UnpinMessage(channel_id: u64, message_id: u64) = DELETE "/channels/{channel_id}/messages/{message_id}/pin";

    // Guilds
    GetAllGuilds query(guild::GetGuildQuery) resp(Vec<models::Guild>) = GET "/guilds";
    CreateGuild body(guild::CreateGuildPayload) resp(models::Guild) = POST "/guilds";
    GetGuild(guild_id: u64) resp(models::Guild) = GET "/guilds/{guild_id}";
    EditGuild(guild_id: u64) body(guild::EditGuildPayload) resp(models::Guild) = PATCH "/guilds/{guild_id}";
    DeleteGuild(guild_id: u64) body(guild::DeleteGuildPayload) = DELETE "/guilds/{guild_id}";

    // Members
    AddBotToGuild(guild_id: u64, bot_id: u64) resp(models::Member) = PUT "/guilds/{guild_id}/bots/{bot_id}";
    GetAllMembers(guild_id: u64) resp(Vec<models::Member>) = GET "/guilds/{guild_id}/members";
    GetAuthenticatedUserAsMember(guild_id: u64) resp(models::Member) = GET "/guilds/{guild_id}/members/me";
    EditAuthenticatedUserAsMember(guild_id: u64)
        body(member::EditClientMemberPayload) resp(models::Member) = PATCH "/guilds/{guild_id}/members/me";
    LeaveGuild(guild_id: u64) = DELETE "/guilds/{guild_id}/members/me";
    GetMember(guild_id: u64, member_id: u64) resp(models::Member) = GET "/guilds/{guild_id}/members/{member_id}";
    EditMember(guild_id: u64, member_id: u64)
        body(member::EditMemberPayload) resp(models::Member) = PATCH "/guilds/{guild_id}/members/{member_id}";
    KickMember(guild_id: u64, member_id: u64) = DELETE "/guilds/{guild_id}/members/{member_id}";

    // Invites
    GetGuildInvites(guild_id: u64) resp(Vec<models::Invite>) = GET "/guilds/{guild_id}/invites";
    CreateInviteToGuild(guild_id: u64)
        body(invite::CreateInvitePayload) resp(models::Invite) = POST "/guilds/{guild_id}/invites";
    DeleteInvite<'a>(guild_id: u64, code: &'a str) = DELETE "/guilds/{guild_id}/invites/{code}";
    GetInvite<'a>(code: &'a str) resp(models::Invite) = GET "/invites/{code}";
    UseInvite<'a>(code: &'a str) query(invite::UseInviteQuery) resp(models::Member) = POST "/invites/{code}";

    // Roles
    EditRolePositions(guild_id: u64) body(Vec<u64>) = PATCH "/guilds/{guild_id}/roles";
    GetAllRoles(guild_id: u64) resp(Vec<models::Role>) = GET "/guilds/{guild_id}/roles";
    CreateRole(guild_id: u64) body(role::CreateRolePayload) resp(models::Role) = POST "/guilds/{guild_id}/roles";
    GetRole(guild_id: u64, role_id: u64) resp(models::Role) = GET "/guilds/{guild_id}/roles/{role_id}";
    EditRole(guild_id: u64, role_id: u64)
        body(role::EditRolePayload) resp(models::Role) = PATCH "/guilds/{guild_id}/roles/{role_id}";
    DeleteRole(guild_id: u64, role_id: u64) = DELETE "/guilds/{guild_id}/roles/{role_id}";

    // Auth
    Login body(auth::LoginRequest) resp(auth::LoginResponse) = POST "/login";

    // Bots
    GetAllBots resp(Vec<models::Bot>) = GET "/bots";
    CreateBot body(user::CreateBotPayload) resp(user::CreateBotResponse) = POST "/bots";
    GetBot(bot_id: u64) resp(models::Bot) = GET "/bots/{bot_id}";
    EditBot(bot_id: u64) resp(models::Bot) = PATCH "/bots/{bot_id}";
    DeleteBot(bot_id: u64) = DELETE "/bots/{bot_id}";
    RegenerateBotToken(bot_id: u64)
        body(user::RegenerateBotTokenPayload) resp(auth::LoginResponse) = POST "/bots/{bot_id}/tokens";

    // Relationships
    GetRelationships resp(Vec<models::Relationship>) = GET "/relationships";
    BlockUser(target_id: u64) resp(models::Relationship) = PUT "/relationships/blocks/{target_id}";
    SendFriendRequest resp(models::Relationship) = POST "/relationships/friends";
    AcceptFriendRequest(target_id: u64) resp(models::Relationship) = PUT "/relationships/friends/{target_id}";
    DeleteRelationship(target_id: u64) = DELETE "/relationships/{target_id}";

    // Users
    CreateUser resp(user::CreateUserResponse) = POST "/users";
    CheckUsernameAvailability<'a>(username: &'a str) = GET "/users/check/{username}";
    GetAuthenticatedUser resp(models::ClientUser) = GET "/users/me";
    EditUser resp(models::ClientUser) = PATCH "/users/me";
    DeleteUser = DELETE "/users/me";
    GetUser(user_id: u64) resp(models::User) = GET "/users/{user_id}";
}

/// Any REST endpoint.
pub trait Endpoint: Copy + Clone + PartialEq + Eq + Send + Sync {
    /// The HTTP method of the endpoint.
    const METHOD: reqwest::Method;
    /// The unformatted path of the endpoint.
    const PATH: &'static str;

    /// The type of query parameters for the endpoint.
    type Query: Serialize + Send + Sync;

    /// The type of body this endpoint expects.
    type Body: Serialize + Send + Sync;

    /// The response this endpoint will return if successful.
    type Response: for<'a> Deserialize<'a> + Send + Sync;

    /// Returns the formatted path of the endpoint as a string, excluding the base URL.
    fn path(&self) -> String;
}
