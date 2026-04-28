use claude_limits_lib::auth::exchange::TokenExchange;
use mockito::Server;

#[tokio::test]
async fn successful_code_exchange() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("grant_type".into(), "authorization_code".into()),
            mockito::Matcher::UrlEncoded("code".into(), "abc".into()),
            mockito::Matcher::UrlEncoded("code_verifier".into(), "verif".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"acc","refresh_token":"ref","expires_in":3600,"token_type":"Bearer"}"#)
        .create_async()
        .await;

    let ex = TokenExchange::with_endpoint(server.url());
    let tok = ex.exchange_code("abc", "verif").await.unwrap();
    assert_eq!(tok.access_token, "acc");
    assert_eq!(tok.refresh_token.as_deref(), Some("ref"));
}

#[tokio::test]
async fn exchange_error_body_surfaces() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .with_status(400)
        .with_body("bad_code")
        .create_async()
        .await;
    let ex = TokenExchange::with_endpoint(server.url());
    let err = ex.exchange_code("abc", "verif").await.unwrap_err();
    assert!(err.to_string().contains("bad_code"));
}

#[tokio::test]
async fn refresh_preserves_refresh_token_when_not_returned() {
    let mut server = Server::new_async().await;
    let _m = server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"access_token":"new","expires_in":3600}"#)
        .create_async()
        .await;
    let ex = TokenExchange::with_endpoint(server.url());
    let tok = ex.refresh("old-refresh").await.unwrap();
    assert_eq!(tok.refresh_token.as_deref(), Some("old-refresh"));
}
