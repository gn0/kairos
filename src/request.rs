use anyhow::{bail, Context, Result};
use reqwest::Response;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{
    policies::ExponentialBackoff, Jitter, RetryTransientMiddleware,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub fn client_with_retry() -> ClientWithMiddleware {
    let policy = ExponentialBackoff::builder()
        .retry_bounds(Duration::from_secs(60), Duration::from_secs(600))
        .jitter(Jitter::Bounded)
        .base(2)
        .build_with_max_retries(30);

    ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(policy))
        .build()
}

pub async fn get(
    url: &str,
    cancellation_token: CancellationToken,
) -> Result<Response> {
    let request = client_with_retry().get(url);

    tokio::select! {
        _ = cancellation_token.cancelled() => {
            bail!("GET: {url}: cancelled")
        }
        response = request.send() => {
            response.with_context(|| format!("GET: {url}"))
        }
    }
}

pub async fn post(
    url: &str,
    form_data: &[(&str, &str)],
    cancellation_token: CancellationToken,
) -> Result<Response> {
    let request = client_with_retry().post(url).form(form_data);

    tokio::select! {
        _ = cancellation_token.cancelled() => {
            bail!("POST: {url}: cancelled")
        }
        response = request.send() => {
            response.with_context(|| format!("POST: {url}"))
        }
    }
}
