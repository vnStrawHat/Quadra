//! The way to the target (specs/19, "M10 in detail"): through each jump host's
//! `direct-tcpip` channel, signed in at every hop; and the local forwards of a connection.

use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Handle};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

use crate::prompt::Prompt;
use crate::session::Checker;
use crate::{Forward, Hop, Options};

const CONNECT: Duration = Duration::from_secs(20);
/// The most jump hosts on a way.
const HOPS: usize = 4;

pub(crate) type Link = Handle<Checker>;

fn said(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Connects to the target through the jump hosts: the target's connection, and the jump
/// hosts' in order, which must live as long as it.
pub(crate) async fn reach(options: &Options, prompt: &Arc<Prompt>) -> Result<(Link, Vec<Link>), String> {
    let mut hops: Vec<Hop> = options.jumps.iter().take(HOPS).cloned().collect();
    hops.push(Hop { target: options.target.clone(), auth: options.auth.clone() });
    let last = hops.len() - 1;
    let mut through: Vec<Link> = Vec::new();
    for (at, hop) in hops.iter().enumerate() {
        let named = |why: String| match at == last {
            true => why,
            false => format!("jump host {}: {why}", hop.target.label()),
        };
        let mut handle = open(hop, through.last(), options, prompt).await.map_err(named)?;
        crate::auth::authenticate(&mut handle, hop, options, prompt).await.map_err(named)?;
        through.push(handle);
    }
    let target = through.pop().ok_or("no target")?;
    Ok((target, through))
}

/// The connection to one hop: by TCP for the first, else through the hop before.
async fn open(
    hop: &Hop,
    before: Option<&Link>,
    options: &Options,
    prompt: &Arc<Prompt>,
) -> Result<Link, String> {
    let (host, port) = (hop.target.host.clone(), hop.target.port);
    let config = Arc::new(client::Config {
        keepalive_interval: Some(Duration::from_secs(30)),
        keepalive_max: 3,
        ..Default::default()
    });
    let checker =
        Checker { host: host.clone(), port, file: options.known_hosts.clone(), prompt: prompt.clone() };
    let refused = |e: russh::Error| match e {
        russh::Error::UnknownKey => "host key verification failed".to_string(),
        e => e.to_string(),
    };
    match before {
        None => {
            let socket = match tokio::time::timeout(CONNECT, TcpStream::connect((host.as_str(), port))).await
            {
                Ok(socket) => socket.map_err(said)?,
                Err(_) => return Err("timed out".into()),
            };
            let _ = socket.set_nodelay(true);
            client::connect_stream(config, socket, checker).await.map_err(refused)
        }
        Some(before) => {
            let channel =
                before.channel_open_direct_tcpip(host, port.into(), "127.0.0.1", 0).await.map_err(said)?;
            client::connect_stream(config, channel.into_stream(), checker).await.map_err(refused)
        }
    }
}

/// Listens for each local forward and carries each connection through a channel; a forward
/// that cannot listen is said in the space. The tasks end when they are aborted.
pub(crate) async fn forward(
    handle: &Arc<Link>,
    forwards: &[Forward],
    prompt: &Prompt,
) -> Vec<JoinHandle<()>> {
    let mut tasks = Vec::new();
    for forward in forwards {
        let listener = match TcpListener::bind(("127.0.0.1", forward.local)).await {
            Ok(listener) => listener,
            Err(e) => {
                prompt.say(&format!("[forward {} could not listen: {e}]\n", forward.label()));
                continue;
            }
        };
        let (handle, forward) = (handle.clone(), forward.clone());
        tasks.push(tokio::spawn(async move {
            while let Ok((mut socket, from)) = listener.accept().await {
                let (handle, forward) = (handle.clone(), forward.clone());
                tokio::spawn(async move {
                    let (host, port, from) = (forward.host, forward.port.into(), from.port().into());
                    if let Ok(channel) = handle.channel_open_direct_tcpip(host, port, "127.0.0.1", from).await
                    {
                        let mut stream = channel.into_stream();
                        let _ = tokio::io::copy_bidirectional(&mut socket, &mut stream).await;
                    }
                });
            }
        }));
    }
    tasks
}
