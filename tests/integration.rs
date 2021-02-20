use osm_admin_hierarchies::{load_tree, run_service, ServiceConfig};
use std::net::TcpListener;

#[actix_rt::test]
async fn locate_hit() {
    // Arrange
    let base_url = spawn_app();
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(&format!("{}/locate?loc=8.822,53.089", &base_url))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    let text = response.text().await.expect("failed to read body");
    assert_eq!(text, "{\"names\":[\"Schwachhausen\"]}");
}

#[actix_rt::test]
async fn locate_miss() {
    // Arrange
    let base_url = spawn_app();
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(&format!("{}/locate?loc=9.822,53.089", &base_url))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    let text = response.text().await.expect("failed to read body");
    assert_eq!(text, "{\"names\":[]}");
}

#[actix_rt::test]
async fn bulk() {
    // Arrange
    let base_url = spawn_app();
    let client = reqwest::Client::new();
    let locs = "1,8.859,53.090\n\
                2,8.822,53.089\n\
                3,0.0,0.0";

    // Act
    let response = client
        .post(&format!("{}/bulk", &base_url))
        .body(locs)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    let text = response.text().await.expect("failed to read body");
    assert_eq!(
        text,
        "1,Schwachhausen\n\
         2,Schwachhausen\n\
         3,\n"
    );
}

fn spawn_app() -> String {
    let path = "./tests/data/schwachhausen.pbf";
    let admin_levels = [10];
    let tree = load_tree(path.into(), &admin_levels).expect("could not build rtree");
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let config = ServiceConfig {
        tree,
        parallel: false,
        listener,
    };
    let server = run_service(config).expect("Failed to start server");
    let _ = tokio::spawn(server);
    format!("http://127.0.0.1:{}", port)
}
