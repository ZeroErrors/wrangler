mod headers;
mod server;
mod setup;
mod watch;

use server::serve;
use setup::{get_preview_id, get_session_id};
use watch::watch_for_changes;

use crate::commands::dev::{socket, ServerConfig};
use crate::settings::toml::Target;

use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime as TokioRuntime;
use url::Url;

/// spin up a local server that routes requests to the preview service
/// that has a Cloudflare Workers runtime without access to zone-specific features
pub fn dev(
    target: Target,
    server_config: ServerConfig,
    verbose: bool,
) -> Result<(), failure::Error> {
    println!("unauthenticated");

    // setup the session
    let session_id = get_session_id()?;

    // upload the initial script
    let preview_id = get_preview_id(
        target.clone(),
        // there is no user for unauthenticated dev
        None,
        &server_config,
        &session_id,
        verbose,
    )?;

    // the local server needs the preview ID to properly route
    // HTTP requests
    //
    // the file watcher updates the preview ID when there is a new
    // Worker
    //
    // Since these run on separate threads, we must stuff the
    // preview ID into an Arc<Mutex so that the server waits on the
    // file watcher to release the lock before routing a request
    let preview_id = Arc::new(Mutex::new(preview_id));
    // a new scope is created to satisfy the borrow checker
    {
        // we must clone each of these variables in order to
        // safely use them in another thread
        let session_id = session_id.clone();
        let preview_id = preview_id.clone();
        let server_config = server_config.clone();
        thread::spawn(move || {
            watch_for_changes(
                target,
                &server_config,
                Arc::clone(&preview_id),
                &session_id,
                verbose,
            )
        });
    }

    let socket_url = get_socket_url(&session_id)?;

    // in order to spawn futures we must create a tokio runtime
    let mut runtime = TokioRuntime::new()?;

    // and we must block the main thread on the completion of
    // said futures
    runtime.block_on(async {
        let devtools_listener = tokio::spawn(socket::listen(socket_url));
        let server = tokio::spawn(serve(server_config, Arc::clone(&preview_id)));
        let res = tokio::try_join!(async { devtools_listener.await? }, async { server.await? });

        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    })
}

fn get_socket_url(session_id: &str) -> Result<Url, url::ParseError> {
    Url::parse(&format!(
        "wss://cloudflareworkers.com/inspect/{}",
        session_id
    ))
}
