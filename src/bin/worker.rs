use resonate::prelude::*;

/// A workflow that blocks on human approval before completing.
///
/// The workflow creates a latent durable promise (`ctx.promise()`) with no
/// function backing it, prints a callback URL containing the promise ID, and
/// then suspends until that promise is resolved externally — by an HTTP
/// request to the gateway. The suspension is durable: if the worker crashes
/// while blocked, another worker recovers the workflow and continues waiting
/// on the same promise.
#[resonate::function]
async fn foo(ctx: &Context, workflow_id: String) -> Result<String> {
    // Latent durable promise — no function backing it. Resolved externally.
    let blocking_promise = ctx.promise::<bool>().create()?;
    let promise_id = blocking_promise.id().await?;

    // Make the promise ID reachable from outside (email, webhook, log, ...).
    ctx.run(send_email, promise_id.clone()).await?;
    println!("blocked, waiting on human interaction (workflow {workflow_id})");

    // Suspend until the promise resolves. Survives crashes.
    let _approved = blocking_promise.await?;
    println!("unblocked, promise resolved");

    Ok(format!("workflow {workflow_id} completed"))
}

/// A leaf function that surfaces the callback URL.
/// In a real system this would send an email, post to Slack, ring a webhook,
/// etc. Here we just print the URL so you can click it from your terminal.
#[resonate::function]
async fn send_email(promise_id: String) -> Result<()> {
    println!("CLICK TO RESOLVE: http://localhost:5001/resolve/{promise_id}");
    Ok(())
}

#[tokio::main]
async fn main() {
    let resonate = Resonate::new(ResonateConfig {
        url: Some("http://localhost:8001".into()),
        group: Some("workers".into()),
        ..Default::default()
    });

    resonate.register(foo).unwrap();
    resonate.register(send_email).unwrap();

    println!("Worker started. Waiting for invocations...");
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl-c");
}
