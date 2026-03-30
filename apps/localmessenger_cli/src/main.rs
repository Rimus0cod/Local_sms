use localmessenger_core::{mvp_blueprint, InviteToken, InviteTransport};
use std::time::Duration;

fn main() {
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "overview".to_string());

    match command.as_str() {
        "overview" => print_overview(),
        "roadmap" => print_roadmap(),
        "invite" => print_invite_demo(),
        _ => print_help(),
    }
}

fn print_overview() {
    let blueprint = mvp_blueprint();

    println!("Local Messenger Blueprint");
    println!("========================");
    println!("Group: {}", blueprint.config.group_name);
    println!("Group id: {}", blueprint.config.group_id);
    println!("Crypto profile: {}", blueprint.config.crypto_profile.label());
    println!("Transport modes:");
    for mode in &blueprint.config.transport_modes {
        println!("  - {}", mode.label());
    }
    println!("Group policy:");
    println!("  - max members: {}", blueprint.config.policy.max_members);
    println!(
        "  - attachment limit: {} MB",
        blueprint.config.policy.max_attachment_size_mb
    );
    println!(
        "  - notifications: {}",
        bool_label(blueprint.config.policy.notifications_enabled)
    );
    println!(
        "  - search: {}",
        bool_label(blueprint.config.policy.search_enabled)
    );
    println!(
        "  - voice notes in MVP: {}",
        bool_label(blueprint.config.policy.voice_notes_enabled)
    );
    println!("Roster size: {}", blueprint.roster.members.len());
    println!("Initial rekey plan: {}", blueprint.initial_rekey_strategy.summary());
}

fn print_roadmap() {
    println!("Next milestones");
    println!("===============");
    for milestone in localmessenger_core::next_milestones() {
        println!("- {milestone}");
    }
}

fn print_invite_demo() {
    let blueprint = mvp_blueprint();
    let invite = InviteToken::ephemeral(
        localmessenger_core::GroupId::new(blueprint.config.group_id.as_str())
            .expect("demo group id should stay valid"),
        "ROOM-2026",
        Duration::from_secs(600),
        1,
        InviteTransport::QrCode,
    )
    .expect("demo invite should be valid");

    println!("Invite demo");
    println!("===========");
    println!("Group id: {}", invite.group_id);
    println!("Transport: {}", invite.transport.label());
    println!("Code: {}", invite.code);
    println!("Uses allowed: {}", invite.max_uses);
}

fn print_help() {
    println!("Usage: cargo run -p localmessenger_cli -- [overview|roadmap|invite]");
}

fn bool_label(value: bool) -> &'static str {
    if value {
        "enabled"
    } else {
        "disabled"
    }
}
