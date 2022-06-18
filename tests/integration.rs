use actix_web::{test, web, App};
use osm_admin_lookup::build_rtree;
use osm_admin_lookup::service::{locate, LocateResponse};
use std::sync::Arc;

#[tokio::test]
async fn locate_400() {
    let path = "./tests/data/schwachhausen.pbf";
    let rtree = build_rtree(path.into(), &[10]).expect("could not build rtree");
    let state = Arc::new(rtree);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(locate),
    )
    .await;
    let req = test::TestRequest::get().uri("/locate?loc=,1").to_request();
    let res = test::call_service(&app, req).await;

    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn locate_hit() {
    let path = "./tests/data/schwachhausen.pbf";
    let rtree = build_rtree(path.into(), &[10]).expect("could not build rtree");
    let state = Arc::new(rtree);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(locate),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/locate?loc=8.822,53.089")
        .to_request();
    let res: LocateResponse = test::call_and_read_body_json(&app, req).await;
    assert_eq!(res.boundaries.len(), 1);
    assert_eq!(res.boundaries[0].name, "Schwachhausen");
    assert_eq!(res.boundaries[0].level, 10);
}

#[tokio::test]
async fn locate_miss() {
    let path = "./tests/data/schwachhausen.pbf";
    let rtree = build_rtree(path.into(), &[10]).expect("could not build rtree");
    let state = Arc::new(rtree);
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(locate),
    )
    .await;
    let req = test::TestRequest::get().uri("/locate?loc=0,0").to_request();
    let res: LocateResponse = test::call_and_read_body_json(&app, req).await;
    assert_eq!(res.boundaries.len(), 0);
}
