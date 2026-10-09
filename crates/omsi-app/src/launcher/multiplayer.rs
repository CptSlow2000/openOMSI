//! The Multiplayer page: two ways to drive with others.
//!
//! * **Connect by Code** - one player hosts (their next duty opens a session; the code to
//!   give out is shown here and on Sessions), the others paste the code. The game finds the
//!   host at home, across the internet through the routers, and through the host's free
//!   Cloudflare tunnel where routers cannot be passed (`omsi_net::bridge`, `ws`, `tunnel`;
//!   cloudflared is fetched by the game when it is not installed). The page says none of
//!   that: the player gives out a code, that is all.
//! * **Servers** - dedicated servers (`omsi --server`, like Minecraft's) added once by their
//!   address and kept in a list with their icon, message of the day, map and players. Joining
//!   one turns the Drive page into the server's: the map, time, date and weather are the
//!   server's, the bus and the duty are the player's, and "Leave Server" goes back.


#[derive(Default)]
pub struct MultiplayerView {
    pub tab: usize,
    pub add_address: String,
    pub add_name: String,
    pub selected: Option<usize>,
    /// 0 Auto, 1 UDP, 2 WebSocket (see `JoinProto`).
    pub proto: usize,
}
