#[actix_rt::test]
async fn locate() {
    // Arrange
    spawn_app();
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get("http://127.0.0.1:8081/locate?loc=8.822,53.089")
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    let text = response.text().await.expect("failed to read body");
    assert_eq!(text, "{\"names\":[\"Schwachhausen\"]}");

    let response = client
        .get("http://127.0.0.1:8081/locate?loc=9.822,53.089")
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    let text = response.text().await.expect("failed to read body");
    assert_eq!(text, "{\"names\":[]}");
}

fn spawn_app() {
    let path = "./tests/data/schwachhausen.pbf";
    let admin_levels = [10];
    let tree =
        rs_geo_playground::load_tree(path.into(), &admin_levels).expect("could not build rtree");
    let config = rs_geo_playground::ServiceConfig {
        tree,
        parallel: false,
        port: 8081,
    };
    let server = rs_geo_playground::run_service(config).expect("Failed to start server");
    let _ = tokio::spawn(server);
}
