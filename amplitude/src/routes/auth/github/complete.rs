use std::time::Duration;

use afire::{HeaderType, Method, Query, Response, Server, SetCookie, Status};
use serde_json::Value;

use crate::{
    database::Database,
    misc::{current_epoch, rand_str, LoginProvider, SESSION_LENGTH},
    session::{GithubSession, Session, SessionPlatform},
    state::State,
};

pub fn attach(server: &mut Server<State>) {
    server.stateful_route(Method::GET, "/auth/github/complete", move |app, req| {
        // Get Code from URI
        let code = match req.query.get("code") {
            Some(i) => i,
            _ => return Response::new().text("No Auth Code Found"),
        };

        // Get and verify state
        let state = match req.query.get("state") {
            Some(i) => i,
            _ => return Response::new().text("No State Found"),
        };
        let state = match app.db().get_oauth(LoginProvider::Github, state) {
            Ok(i) => i,
            Err(_) => return Response::new().text("Invalid state"),
        };

        if current_epoch() - state >= 60 * 10 {
            return Response::new().text("State Expired");
        }

        // Get Access Token
        let cfg = app.config.auth.github_oauth.as_ref().unwrap();
        let resp = ureq::post("https://github.com/login/oauth/access_token")
            .query("client_secret", &cfg.app_secret)
            .query("client_id", &cfg.app_id)
            .query("code", code)
            .timeout(Duration::from_secs(app.config.server.req_duration))
            .call()
            .unwrap()
            .into_string()
            .unwrap();

        // Parse Response and net Token
        let token = Query::from_body(&resp);
        let token = token.get("access_token").expect("No Access Token!?");

        // Get User Info
        let user_raw = ureq::get("https://api.github.com/user")
            .set("Authorization", &format!("token {token}"))
            .call()
            .unwrap()
            .into_reader();

        // Parse Jason
        let user: Value = serde_json::from_reader(user_raw).unwrap();
        let id = user.get("id").unwrap().as_u64().unwrap();
        let login = user.get("login").unwrap().as_str().unwrap();
        let name = user.get("name").unwrap().as_str().unwrap_or(login);
        let avatar = user.get("avatar_url").unwrap().as_str().unwrap();

        let github = GithubSession {
            github_id: id.to_string(),
            login: login.to_owned(),
            token: token.to_owned(),
        };
        let token = rand_str(10);
        let session = Session {
            platform: SessionPlatform::Github(github),
            token: token.to_owned(),
            id: rand_str(10),
            name: name.to_owned(),
            avatar: avatar.to_owned(),
            signup: current_epoch(),
        };

        app.db()
            .add_session(&session, req.headers.get(HeaderType::UserAgent))
            .unwrap();

        let cookie = SetCookie::new("session", token)
            .path("/")
            .max_age(SESSION_LENGTH);

        Response::new()
            .status(Status::TemporaryRedirect)
            .header("Cache-Control", "no-store")
            .header("Location", "/")
            .cookie(cookie)
    });
}
