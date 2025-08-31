use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{
    policies::ExponentialBackoff, Jitter, RetryTransientMiddleware,
};
use std::time::Duration;

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
