use aws_lambda_runtime_proxy::{LambdaRuntimeApiClient, Proxy};
use env_logger::Env;
use log::debug;
use tokio::process::Command;

// mode bit flags
const AFTER_RESPONSE: usize = 1 << 0;
const AFTER_ERROR: usize = 1 << 1;

#[tokio::main]
async fn main() {
  let env = Env::default().filter_or("AWS_LAMBDA_POST_RUNNER_LOG_LEVEL", "error");
  env_logger::init_from_env(env);

  let cmd = std::env::var("AWS_LAMBDA_POST_RUNNER_COMMAND")
    .expect("No command found for aws-lambda-post-runner");
  debug!("got AWS_LAMBDA_POST_RUNNER_COMMAND: {}", cmd);

  let mode = std::env::var("AWS_LAMBDA_POST_RUNNER_MODE")
    .map(|mode| {
      debug!("got AWS_LAMBDA_POST_RUNNER_MODE: {}", mode);
      mode
        .split(',')
        .map(|m| match m {
          "AfterResponse" => AFTER_RESPONSE,
          "AfterError" => AFTER_ERROR,
          _ => panic!("Invalid mode for aws-lambda-post-runner: {}", m),
        })
        .fold(0, |acc, m| acc | m)
    })
    // default to all modes
    .unwrap_or(usize::MAX);
  debug!("parsed AWS_LAMBDA_POST_RUNNER_MODE: {}", mode);

  Proxy::default()
    .spawn()
    .await
    .server
    .serve(move |req| {
      let cmd = cmd.clone();

      async move {
        let path = req.uri().path();
        debug!("got runtime api request: {}", path);

        let need_exec = (mode & AFTER_RESPONSE != 0
          && path.starts_with("/2018-06-01/runtime/invocation/")
          && path.ends_with("/response"))
          || (mode & AFTER_ERROR != 0
            && path.starts_with("/2018-06-01/runtime/invocation/")
            && path.ends_with("/error"));

        // forward the request
        let res = LambdaRuntimeApiClient::forward(req).await;

        if need_exec {
          debug!("executing AWS_LAMBDA_POST_RUNNER_COMMAND: {}", cmd);

          // before proceed, run the command
          Command::new("/bin/bash")
            .arg("-c")
            .arg(&cmd)
            .spawn()
            .unwrap()
            .wait()
            .await
            .unwrap();

          debug!("finished executing AWS_LAMBDA_POST_RUNNER_COMMAND: {}", cmd);
        }

        res
      }
    })
    .await
}
