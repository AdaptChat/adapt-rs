macro_rules! endpoints {
    ($(
        $(#[$doc:meta])* $name:ident $(($($params:ident: $ty:ty),+))? = $method:ident $path:literal;
    )+) => {
        $(
            paste::paste! {
                pub const [<$name:snake:upper>]: __types::$name = __types::$name {
                    $($($params: None),+)?
                };
            }
        )+
        pub mod __types {
            $(
                $(#[$doc])*
                #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
                pub struct $name { $($(pub(super) $params: Option<$ty>),+)? }

                $(
                    impl $name {
                        $(
                            #[inline]
                            #[must_use]
                            #[doc = concat!("Sets the `", stringify!($params), "` path parameter.")]
                            pub fn $params(mut self, $params: $ty) -> Self {
                                self.$params = Some($params);
                                self
                            }
                        )+
                    }
                )?

                impl super::Endpoint for $name {
                    const METHOD: reqwest::Method = reqwest::Method::$method;
                    const PATH: &'static str = $path;

                    #[inline]
                    fn path(&self) -> String {
                        format!($path, $($($params = self.$params.unwrap()),+)?)
                    }
                }
            )+
        }
    };
}

endpoints! {
    // Channels
    GetChannel(channel_id: u64) = GET "/channels/{channel_id}";
    EditChannel(channel_id: u64) = PATCH "/channels/{channel_id}";
    DeleteChannel(channel_id: u64) = DELETE "/channels/{channel_id}";
    GetGuildChannels(guild_id: u64) = GET "/guilds/{guild_id}/channels";
    CreateGuildChannel(guild_id: u64) = POST "/guilds/{guild_id}/channels";
    // Guilds
    GetAllGuilds = GET "/guilds";
    CreateGuild = POST "/guilds";
    GetGuild(guild_id: u64) = GET "/guilds/{guild_id}";
    EditGuild(guild_id: u64) = PATCH "/guilds/{guild_id}";
    DeleteGuild(guild_id: u64) = DELETE "/guilds/{guild_id}";
    // Roles
    GetGuildRoles(guild_id: u64) = GET "/guilds/{guild_id}/roles";
    CreateRole(guild_id: u64) = POST "/guilds/{guild_id}/roles";
    GetRole(guild_id: u64, role_id: u64) = GET "/guilds/{guild_id}/roles/{role_id}";
    EditRole(guild_id: u64, role_id: u64) = PATCH "/guilds/{guild_id}/roles/{role_id}";
    DeleteRole(guild_id: u64, role_id: u64) = DELETE "/guilds/{guild_id}/roles/{role_id}";
    // Auth
    Login = POST "/login";
    // Users
    CreateUser = POST "/users";
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
