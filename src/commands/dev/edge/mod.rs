mod server;
mod setup;

use server::serve;
use setup::Session;

use crate::commands::dev::ServerConfig;
use crate::settings::global_user::GlobalUser;
use crate::settings::toml::{DeployConfig, Target};

use tokio::runtime::Runtime as TokioRuntime;

pub fn dev(
    target: Target,
    user: GlobalUser,
    server_config: ServerConfig,
    deploy_config: DeployConfig,
    verbose: bool,
) -> Result<(), failure::Error> {
    let session = Session::new(&target, &user, &deploy_config)?;
    let mut target = target;

    let preview_token = setup::upload(
        &mut target,
        &deploy_config,
        &user,
        session.preview_token,
        verbose,
    )?;

    let server = serve(server_config, preview_token, session.host);
    let mut runtime = TokioRuntime::new()?;
    runtime.block_on(server)
}
