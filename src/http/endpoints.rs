macro_rules! endpoints {
    ($(
        $(#[$doc:meta])* $name:ident $(<$($lt:lifetime),+>)? $(($($params:ident: $ty:ty),+))?
        = $method:ident $path:literal;
    )+) => {
        $(
            $(#[$doc])*
            #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
            pub struct $name $(<$($lt),+>)? $({ $(pub $params: $ty),+ })?;

            impl Endpoint for $name {
                const METHOD: reqwest::Method = reqwest::Method::$method;
                const PATH: &'static str = $path;

                #[inline]
                fn path(&self) -> String {
                    format!($path, $($($params = self.$params),+)?)
                }
            }
        )+
    }
}

endpoints! {
    // Channels
    GetChannel(channel_id: u64) = GET "/channels/{channel_id}";
    EditChannel(channel_id: u64) = PATCH "/channels/{channel_id}";
    DeleteChannel(channel_id: u64) = DELETE "/channels/{channel_id}";
    GetGuildChannels(guild_id: u64) = GET "/guilds/{guild_id}/channels";
    CreateGuildChannel(guild_id: u64) = POST "/guilds/{guild_id}/channels";

    // Messages
    GetMessageHistory(channel_id: u64) = GET "/channels/{channel_id}/messages";
    CreateMessage(channel_id: u64) = POST "/channels/{channel_id}/messages";
    GetMessage(channel_id: u64, message_id: u64) = GET "/channels/{channel_id}/messages/{message_id}";
    EditMessage(channel_id: u64, message_id: u64) = PATCH "/channels/{channel_id}/messages/{message_id}";
    DeleteMessage(channel_id: u64, message_id: u64) = DELETE "/channels/{channel_id}/messages/{message_id}";
    PinMessage(channel_id: u64, message_id: u64) = PUT "/channels/{channel_id}/messages/{message_id}/pin";
    UnpinMessage(channel_id: u64, message_id: u64) = DELETE "/channels/{channel_id}/messages/{message_id}/pin";

    // Guilds
    GetAllGuilds = GET "/guilds";
    CreateGuild = POST "/guilds";
    GetGuild(guild_id: u64) = GET "/guilds/{guild_id}";
    EditGuild(guild_id: u64) = PATCH "/guilds/{guild_id}";
    DeleteGuild(guild_id: u64) = DELETE "/guilds/{guild_id}";

    // Members
    AddBotToGuild(guild_id: u64, bot_id: u64) = PUT "/guilds/{guild_id}/bots/{bot_id}";
    GetAllMembers(guild_id: u64) = GET "/guilds/{guild_id}/members";
    GetAuthenticatedUserAsMember(guild_id: u64) = GET "/guilds/{guild_id}/members/me";
    EditAuthenticatedUserAsMember(guild_id: u64) = PATCH "/guilds/{guild_id}/members/me";
    LeaveGuild(guild_id: u64) = DELETE "/guilds/{guild_id}/members/me";
    GetMember(guild_id: u64, member_id: u64) = GET "/guilds/{guild_id}/members/{member_id}";
    EditMember(guild_id: u64, member_id: u64) = PATCH "/guilds/{guild_id}/members/{member_id}";
    KickMember(guild_id: u64, member_id: u64) = DELETE "/guilds/{guild_id}/members/{member_id}";

    // Invites
    GetGuildInvites(guild_id: u64) = GET "/guilds/{guild_id}/invites";
    CreateInviteToGuild(guild_id: u64) = POST "/guilds/{guild_id}/invites";
    DeleteInvite<'a>(guild_id: u64, code: &'a str) = DELETE "/guilds/{guild_id}/invites/{code}";
    GetInvite<'a>(code: &'a str) = GET "/invites/{code}";
    UseInvite<'a>(code: &'a str) = POST "/invites/{code}";

    // Roles
    EditRolePositions(guild_id: u64) = PATCH "/guilds/{guild_id}/roles";
    GetAllRoles(guild_id: u64) = GET "/guilds/{guild_id}/roles";
    CreateRole(guild_id: u64) = POST "/guilds/{guild_id}/roles";
    GetRole(guild_id: u64, role_id: u64) = GET "/guilds/{guild_id}/roles/{role_id}";
    EditRole(guild_id: u64, role_id: u64) = PATCH "/guilds/{guild_id}/roles/{role_id}";
    DeleteRole(guild_id: u64, role_id: u64) = DELETE "/guilds/{guild_id}/roles/{role_id}";

    // Auth
    Login = POST "/login";

    // Bots
    GetAllBots = GET "/bots";
    CreateBot = POST "/bots";
    GetBot(bot_id: u64) = GET "/bots/{bot_id}";
    EditBot(bot_id: u64) = PATCH "/bots/{bot_id}";
    DeleteBot(bot_id: u64) = DELETE "/bots/{bot_id}";
    RegenerateBotToken(bot_id: u64) = POST "/bots/{bot_id}/tokens";

    // Relationships
    GetRelationships = GET "/relationships";
    BlockUser(target_id: u64) = PUT "/relationships/blocks/{target_id}";
    SendFriendRequest = POST "/relationships/friends";
    AcceptFriendRequest(target_id: u64) = PUT "/relationships/friends/{target_id}";
    DeleteRelationship(target_id: u64) = DELETE "/relationships/{target_id}";

    // Users
    CreateUser = POST "/users";
    CheckUsernameAvailability<'a>(username: &'a str) = GET "/users/check/{username}";
    GetAuthenticatedUser = GET "/users/me";
    EditUser = PATCH "/users/me";
    DeleteUser = DELETE "/users/me";
    GetUser(user_id: u64) = GET "/users/{user_id}";
}

/// Any REST endpoint.
pub trait Endpoint {
    /// The HTTP method of the endpoint.
    const METHOD: reqwest::Method;
    /// The unformatted path of the endpoint.
    const PATH: &'static str;

    /// Returns the formatted path of the endpoint as a string, excluding the base URL.
    fn path(&self) -> String;
}
