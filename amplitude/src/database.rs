use rusqlite::{params, Connection};
use tracing::{error, info};

use crate::{
    misc::{current_epoch, LoginProvider, SESSION_LENGTH},
    session::{GithubSession, GoogleSession, Session, SessionPlatform},
};

type SessionMeta = (String, u64, Option<String>);

// Increment every time schema changes, even in dev
const DATABASE_VERSION: u64 = 1;

pub trait Database {
    // == Base ==
    fn init(&mut self) -> anyhow::Result<()>;
    fn cleanup(&mut self) -> anyhow::Result<()>;
    fn garbage_collect(&mut self) -> anyhow::Result<()>;

    // == Auth ==
    fn add_oauth(&self, service: LoginProvider, state: &str) -> anyhow::Result<()>;
    fn get_oauth(&self, service: LoginProvider, state: &str) -> anyhow::Result<u64>;

    // == Session ==
    fn add_session(&self, session: &Session, agent: Option<&str>) -> anyhow::Result<()>;
    fn get_session(&self, token: &str) -> anyhow::Result<Session>;
    fn delete_session(&self, token: &str) -> anyhow::Result<()>;
    fn delete_sessions(&self, session: &Session) -> anyhow::Result<()>;
    fn get_sessions(&self, session: &Session) -> anyhow::Result<Vec<SessionMeta>>;
}

impl Database for Connection {
    fn init(&mut self) -> anyhow::Result<()> {
        self.pragma_update(None, "journal_mode", "WAL")?;
        self.pragma_update(None, "synchronous", "NORMAL")?;

        let db_version =
            self.pragma_query_value(None, "user_version", |row| row.get::<_, u64>(0))?;

        match db_version {
            DATABASE_VERSION => info!("Loaded database at `{}`", self.path().unwrap()),
            0 => {
                info!("Creating database at `{}`", self.path().unwrap());
                self.pragma_update(None, "user_version", DATABASE_VERSION)?;
            }
            i => {
                error!(
                    "Database version mismatch. Expected {}, got {}",
                    DATABASE_VERSION, i
                );
            }
        }

        let trans = self.transaction()?;
        for i in [
            include_str!("./sql/auth/github/create_users.sql"),
            include_str!("./sql/auth/github/create_oauth_state.sql"),
            include_str!("./sql/auth/google/create_users.sql"),
            include_str!("./sql/auth/google/create_oauth_state.sql"),
            include_str!("./sql/create_sessions.sql"),
        ] {
            trans.execute(i, [])?;
        }
        trans.commit()?;

        Ok(())
    }

    fn cleanup(&mut self) -> anyhow::Result<()> {
        self.garbage_collect()?;
        self.pragma_update(None, "optimize", "")?;
        self.pragma_update(None, "wal_checkpoint", "TRUNCATE")?;
        Ok(())
    }

    fn garbage_collect(&mut self) -> anyhow::Result<()> {
        let cutoff = current_epoch() - 60 * 60; // (one hour)
        let trans = self.transaction()?;

        for i in [
            include_str!("./sql/auth/github/delete_oauth.sql"),
            include_str!("./sql/auth/google/delete_oauth.sql"),
        ] {
            trans.execute(i, [cutoff])?;
        }
        trans.commit()?;

        Ok(())
    }

    fn add_oauth(&self, service: LoginProvider, state: &str) -> anyhow::Result<()> {
        match service {
            LoginProvider::Github => self.execute(
                "INSERT INTO github_oauth_state (state, created) VALUES (?1, strftime('%s','now'))",
                [state],
            ),
            LoginProvider::Google => self.execute(
                "INSERT INTO google_oauth_state (state, created) VALUES (?1, strftime('%s','now'))",
                [state],
            ),
        }?;

        Ok(())
    }

    /// Gets and removes the oauth state
    fn get_oauth(&self, service: LoginProvider, state: &str) -> anyhow::Result<u64> {
        let res = match service {
            LoginProvider::Github => {
                let date = self.query_row(
                    "SELECT created FROM github_oauth_state WHERE state = ?1",
                    [state],
                    |x| x.get::<_, u64>(0),
                )?;
                self.execute("DELETE FROM github_oauth_state WHERE state = ?1", [state])?;
                date
            }
            LoginProvider::Google => {
                let date = self.query_row(
                    "SELECT created FROM google_oauth_state WHERE state = ?1",
                    [state],
                    |x| x.get::<_, u64>(0),
                )?;
                self.execute("DELETE FROM google_oauth_state WHERE state = ?1", [state])?;
                date
            }
        };

        Ok(res)
    }

    fn add_session(&self, session: &Session, agent: Option<&str>) -> anyhow::Result<()> {
        let id = match &session.platform {
            SessionPlatform::Github(p) => self.query_row(
                include_str!("./sql/auth/github/upsert_login.sql"),
                params![
                    session.id,
                    p.github_id,
                    session.name,
                    p.login,
                    session.avatar,
                    p.token
                ],
                |x| x.get::<_, String>(0),
            ),
            SessionPlatform::Google(p) => self.query_row(
                include_str!("./sql/auth/google/upsert_login.sql"),
                params![
                    session.id,
                    p.google_id,
                    session.name,
                    session.avatar,
                    p.access_token,
                ],
                |x| x.get::<_, String>(0),
            ),
        }
        .unwrap_or_else(|_| session.id.to_owned());

        self.execute(
            include_str!("./sql/insert_sessions.sql"),
            params![
                id,
                session.token,
                session.platform.as_provider() as u8,
                agent
            ],
        )?;

        Ok(())
    }

    fn get_session(&self, token: &str) -> anyhow::Result<Session> {
        let (created, user_id, platform) = self.query_row(
            "SELECT created, user_id, platform FROM sessions WHERE session_id = ?",
            [token],
            |x| {
                Ok((
                    x.get::<_, u64>(0)?,
                    x.get::<_, String>(1)?,
                    x.get::<_, u8>(2)?,
                ))
            },
        )?;

        // Expire session after 30 days
        if current_epoch() - created > SESSION_LENGTH {
            self.delete_session(token)?;
            return Err(anyhow::anyhow!("Session expired"));
        }

        Ok(match platform.into() {
            LoginProvider::Github => {
                self.query_row("SELECT * FROM github_users WHERE id = ?1", [user_id], |x| {
                    Ok(Session {
                        id: x.get::<_, String>(0)?,
                        name: x.get::<_, String>(2)?,
                        avatar: x.get::<_, String>(4)?,
                        signup: x.get::<_, u64>(6)?,
                        token: token.to_string(),
                        platform: SessionPlatform::Github(GithubSession {
                            github_id: x.get::<_, String>(1)?,
                            login: x.get::<_, String>(3)?,
                            token: x.get::<_, String>(5)?,
                        }),
                    })
                })?
            }
            LoginProvider::Google => {
                self.query_row("SELECT * FROM google_users WHERE id = ?1", [user_id], |x| {
                    Ok(Session {
                        id: x.get::<_, String>(0)?,
                        name: x.get::<_, String>(2)?,
                        avatar: x.get::<_, String>(3)?,
                        signup: x.get::<_, u64>(5)?,
                        token: token.to_string(),
                        platform: SessionPlatform::Google(GoogleSession {
                            google_id: x.get::<_, String>(1)?,
                            access_token: x.get::<_, String>(4)?,
                        }),
                    })
                })?
            }
        })
    }

    fn delete_session(&self, token: &str) -> anyhow::Result<()> {
        self.execute("DELETE FROM sessions WHERE session_id = ?1", [token])?;
        Ok(())
    }

    fn delete_sessions(&self, session: &Session) -> anyhow::Result<()> {
        self.execute("DELETE FROM sessions WHERE user_id = ?1", [&session.id])?;
        Ok(())
    }

    fn get_sessions(&self, session: &Session) -> anyhow::Result<Vec<SessionMeta>> {
        let mut stmt = self
            .prepare("SELECT session_id, created, user_agent FROM sessions WHERE user_id = ?1")?;

        let sessions = stmt
            .query_map([&session.id], |x| {
                Ok((
                    x.get::<_, String>(0)?,
                    x.get::<_, u64>(1)?,
                    x.get::<_, Option<String>>(2)?,
                ))
            })?
            .map(Result::unwrap)
            .collect::<Vec<_>>();

        Ok(sessions)
    }
}
