#[actix_rt::test]
async fn locate() {
    // Arrange
    spawn_app();
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get("http://127.0.0.1:8081/locate?loc=12.533869297330039,52.157853041951206")
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    let text = response.text().await.expect("failed to read body");
    assert_eq!(
        text,
        "{\"names\":[\"Brandenburg\",\"Potsdam-Mittelmark\",\"Bad Belzig\",\"Lübnitz\"]}"
    );
}

fn spawn_app() {
    let config = rs_geo_playground::ServiceConfig {
        bin_path: "./brandenburg-rtree.bin".into(),
        parallel: false,
        port: 8081,
    };
    let server = rs_geo_playground::run_service(config).expect("Failed to start server");
    let _ = tokio::spawn(server);
}
