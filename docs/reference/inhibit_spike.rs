// THROWAWAY SPIKE: take a logind inhibitor lock, hold the fd, release on SIGTERM/SIGINT.
use signal_hook::{consts::{SIGINT, SIGTERM}, iterator::Signals};
use zbus::{Connection, OwnedFd};

async fn take_lock() -> zbus::Result<(Connection, OwnedFd)> {
    let conn = Connection::system().await?;
    let reply = conn
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "Inhibit",
            &(
                "sleep:idle:handle-lid-switch",
                "coffeegent-spike",
                "rust spike",
                "block",
            ),
        )
        .await?;
    let fd: OwnedFd = reply.body().deserialize()?;
    Ok((conn, fd))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Both are kept alive until main returns; dropping the fd releases the lock.
    let (_conn, _fd) = zbus::block_on(take_lock())?;
    println!(
        "pid {} holding sleep:idle:handle-lid-switch",
        std::process::id()
    );
    let mut signals = Signals::new([SIGTERM, SIGINT])?;
    if let Some(sig) = signals.forever().next() {
        println!("got signal {sig}, releasing lock");
    }
    Ok(())
}
