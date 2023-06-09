use config_file::FromConfigFile;
use salvo::cors::Cors;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
mod entity;
use jsonwebtoken::{self, EncodingKey};
use salvo::jwt_auth::HeaderFinder;
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, Database,
    DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter, Statement,
};
use std::sync::OnceLock;
use time::{Duration, OffsetDateTime};

use entity::{host_tb, prelude::*, project_tb};

use chrono::prelude::*;

use salvo::http::Method;

#[derive(Deserialize, Clone)]
struct Admin {
    name: String,
    pass: String,
}

#[derive(Deserialize, Clone)]
struct Config {
    host: String,
    db_url: String,
    secret_key: String,
    admin: Admin,
}

struct CatchError(anyhow::Error);

impl<T: Into<anyhow::Error>> From<T> for CatchError {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

#[async_trait]
impl Writer for CatchError {
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        res.status_code(StatusCode::BAD_REQUEST);
        let j = serde_json::json!({
            "status":400,
            "msg":self.0.to_string()
        });
        res.render(Text::Json(j.to_string()));
    }
}

static DB_CONN: OnceLock<DatabaseConnection> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    username: String,
    exp: i64,
}

struct AuthorGuard;

#[handler]
impl AuthorGuard {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) -> Result<(), CatchError> {
        if req.method() == Method::OPTIONS {
            ctrl.call_next(req, depot, res).await;
            return Ok(());
        }
        match depot.jwt_auth_state() {
            JwtAuthState::Authorized => {
                //println!("oK");
                ctrl.call_next(req, depot, res).await;
            }
            _ => {
                //println!("Unauthorized");
                let j = serde_json::json!({
                    "status":401,
                    "msg":"Unauthorized"
                });
                res.status_code(StatusCode::UNAUTHORIZED);
                res.render(Text::Plain(j.to_string()));
                ctrl.skip_rest();
            }
        }
        Ok(())
    }
}

struct Login {
    admin: Admin,
    secret_key: String,
}

#[handler]
impl Login {
    async fn handle(
        &self,
        req: &mut Request,
        _depot: &mut Depot,
        res: &mut Response,
    ) -> Result<(), CatchError> {
        let name = req
            .form::<String>("name")
            .await
            .ok_or(anyhow::anyhow!("name not found in the request"))?;
        let pass = req
            .form::<String>("pass")
            .await
            .ok_or(anyhow::anyhow!("pass not found in the request"))?;
        if self.admin.name == name && self.admin.pass == pass {
            let exp = OffsetDateTime::now_utc() + Duration::days(1);
            let claim = JwtClaims {
                username: format!("{name}"),
                exp: exp.unix_timestamp(),
            };
            let token = jsonwebtoken::encode(
                &jsonwebtoken::Header::default(),
                &claim,
                &EncodingKey::from_secret(self.secret_key.as_bytes()),
            )?;
            let j = serde_json::json!({
                "status":200,
                "msg":{
                    "token":token
                }
            });
            res.render(Text::Json(j.to_string()));
        } else {
            let j = serde_json::json!({
                "status":400,
                "msg":"用户名或密码错误"
            });
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Json(j.to_string()));
        }
        Ok(())
    }
}

#[handler]
async fn add_host(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), CatchError> {
    let host = req
        .form::<String>("host")
        .await
        .ok_or(anyhow::anyhow!("host not found in the request"))?
        .trim()
        .to_owned();
    let secret = req
        .form::<String>("secret")
        .await
        .ok_or(anyhow::anyhow!("secret not found in the request"))?
        .trim()
        .to_owned();
    let protocol = req
        .form::<String>("protocol")
        .await
        .ok_or(anyhow::anyhow!("protocol not found in the request"))?
        .trim()
        .to_owned();
    let db = DB_CONN.get().ok_or(anyhow::anyhow!("database is busy"))?;
    let search = HostTb::find()
        .filter(host_tb::Column::Host.eq(&host))
        .one(db)
        .await?;
    if !search.is_some() {
        let mut insert = host_tb::ActiveModel::new();
        let time_now = Local::now();
        insert.create_time = ActiveValue::set(Some(time_now.naive_local()));
        insert.update_time = ActiveValue::set(Some(time_now.naive_local()));
        insert.host = ActiveValue::set(host);
        insert.secret = ActiveValue::set(secret);
        insert.protocol = ActiveValue::set(protocol);
        insert.insert(db).await?;
        let j = serde_json::json!({
            "status":200,
            "msg":{
                "msg":"Ok"
            }
        });
        res.render(Text::Json(j.to_string()));
    } else {
        let j = serde_json::json!({
            "status":400,
            "msg":format!("{host} already exists in database")
        });
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Text::Json(j.to_string()));
    }
    Ok(())
}

#[handler]
async fn del_host(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), CatchError> {
    let host_id = req
        .form::<u64>("id")
        .await
        .ok_or(anyhow::anyhow!("host id not found in the request"))?;
    let db = DB_CONN.get().ok_or(anyhow::anyhow!("database is busy"))?;
    let search = HostTb::find()
        .filter(host_tb::Column::Id.eq(host_id))
        .one(db)
        .await?;
    if search.is_some() {
        host_tb::ActiveModel::from(search.unwrap())
            .delete(db)
            .await?;
        let sql = r#"delete from project_tb where parent_id = ?"#;
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::MySql,
            sql,
            [sea_orm::Value::BigInt(Some(host_id as i64))],
        );
        db.query_one(stmt).await?;
        let j = serde_json::json!({
            "status":200,
            "msg":{
                "msg":"OK"
            }
        });
        res.render(Text::Json(j.to_string()));
    } else {
        let j = serde_json::json!({
            "status":400,
            "msg":"record not found in the database"
        });
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Text::Json(j.to_string()));
    }
    Ok(())
}

#[handler]
async fn edit_host(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), CatchError> {
    let host_id = req
        .form::<u64>("id")
        .await
        .ok_or(anyhow::anyhow!("host_id not found in the request"))?;
    let secret = req
        .form::<String>("secret")
        .await
        .ok_or(anyhow::anyhow!("secret not found in the request"))?
        .trim()
        .to_owned();
    let host = req
        .form::<String>("host")
        .await
        .ok_or(anyhow::anyhow!("host not found in the request"))?
        .trim()
        .to_owned();
    let protocol = req
        .form::<String>("protocol")
        .await
        .ok_or(anyhow::anyhow!("protocol not found in the request"))?
        .trim()
        .to_owned();
    let db = DB_CONN.get().ok_or(anyhow::anyhow!("database is busy"))?;
    let search = HostTb::find()
        .filter(host_tb::Column::Id.eq(host_id))
        .one(db)
        .await?;
    if search.is_none() {
        let j = serde_json::json!({
            "status":400,
            "msg":"record not found in the database"
        });
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Text::Json(j.to_string()));
    } else {
        let mut active = host_tb::ActiveModel::from(search.unwrap());
        active.host = ActiveValue::set(host);
        active.protocol = ActiveValue::set(protocol);
        active.secret = ActiveValue::set(secret);
        active.update_time = ActiveValue::set(Some(Local::now().naive_local()));
        active.update(db).await?;
        let j = serde_json::json!({
            "status":200,
            "msg":{
                "msg":"Ok"
            }
        });
        res.render(Text::Json(j.to_string()));
    }
    Ok(())
}

#[handler]
async fn host_list(
    _req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), CatchError> {
    //println!("invoke");
    let db = DB_CONN.get().ok_or(anyhow::anyhow!("database is busy"))?;
    let mut r = HostTb::find().into_json().all(db).await?;
    for item in &mut r {
        let id = item
            .get("id")
            .ok_or(anyhow::anyhow!("id not found in the data object"))?
            .as_u64()
            .ok_or(anyhow::anyhow!("id is invalid u64"))?;
        let list = ProjectTb::find()
            .filter(project_tb::Column::ParentId.eq(id))
            .into_json()
            .all(db)
            .await?;
        item["projects"] = serde_json::Value::Array(list);
    }
    let j = serde_json::json!({
        "status":200,
        "msg":{
            "list":r
        }
    });
    res.render(Text::Json(j.to_string()));
    Ok(())
}

#[handler]
async fn add_project(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), CatchError> {
    let path = req
        .form::<String>("path")
        .await
        .ok_or(anyhow::anyhow!("path not found in the request"))?
        .trim()
        .to_owned();

    let name = req
        .form::<String>("name")
        .await
        .ok_or(anyhow::anyhow!("name not found in the request"))?
        .trim()
        .to_owned();

    let parent_id = req
        .form::<i32>("parent_id")
        .await
        .ok_or(anyhow::anyhow!("parent_id not found in the request"))?;

    let token = format!("{:x}", md5::compute(uuid::Uuid::new_v4().to_string()));
    let db = DB_CONN.get().ok_or(anyhow::anyhow!("database is busy"))?;
    let search = HostTb::find()
        .filter(host_tb::Column::Id.eq(parent_id))
        .one(db)
        .await?;
    if search.is_none() {
        let j = serde_json::json!({
            "status":400,
            "msg":"group does not exist"
        });
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Text::Json(j.to_string()));
    } else {
        let mut info = project_tb::ActiveModel::new();
        let now = Local::now().naive_local();
        info.create_time = ActiveValue::set(Some(now.clone()));
        info.update_time = ActiveValue::set(Some(now.clone()));
        info.path = ActiveValue::set(path);
        info.token = ActiveValue::set(token);
        info.name = ActiveValue::set(name);
        info.parent_id = ActiveValue::set(parent_id);
        info.insert(db).await?;
        let j = serde_json::json!({
            "status":200,
            "msg":{
                "msg":"Ok"
            }
        });
        res.render(Text::Json(j.to_string()));
    }
    Ok(())
}

#[handler]
async fn del_project(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), CatchError> {
    let id = req
        .form::<i32>("id")
        .await
        .ok_or(anyhow::anyhow!("id not found in the request"))?;

    let db = DB_CONN.get().ok_or(anyhow::anyhow!("database is busy"))?;

    let r = ProjectTb::delete_by_id(id).exec(db).await?;
    let j = serde_json::json!({
        "status":200,
        "msg":{
            "msg":format!("Affect {} row",r.rows_affected)
        }
    });
    res.render(Text::Json(j.to_string()));
    Ok(())
}
#[handler]
async fn edit_project(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), CatchError> {
    let id = req
        .form::<i32>("id")
        .await
        .ok_or(anyhow::anyhow!("id not found in the request"))?;

    let path = req
        .form::<String>("path")
        .await
        .ok_or(anyhow::anyhow!("path not found in the request"))?
        .trim()
        .to_owned();

    let name = req
        .form::<String>("name")
        .await
        .ok_or(anyhow::anyhow!("name not found in the request"))?
        .trim()
        .to_owned();

    let parent_id = req
        .form::<i32>("parent_id")
        .await
        .ok_or(anyhow::anyhow!("parent_id not found in the request"))?;

    let token = req
        .form::<String>("token")
        .await
        .ok_or(anyhow::anyhow!("token not found in the request"))?
        .trim()
        .to_owned();

    let db = DB_CONN.get().ok_or(anyhow::anyhow!("database is busy"))?;
    let info = ProjectTb::find()
        .filter(project_tb::Column::Id.eq(id))
        .one(db)
        .await?;
    if info.is_none() {
        let j = serde_json::json!({
            "status":400,
            "msg":"record not found in the database"
        });
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Text::Json(j.to_string()));
    } else {
        let mut info = project_tb::ActiveModel::from(info.unwrap());
        info.path = ActiveValue::set(path);
        info.parent_id = ActiveValue::set(parent_id);
        info.name = ActiveValue::set(name);
        info.update_time = ActiveValue::set(Some(Local::now().naive_local()));
        info.token = ActiveValue::set(token);
        info.update(db).await?;
        let j = serde_json::json!({
            "status":200,
            "msg":{
                "msg":"OK"
            }
        });
        res.render(Text::Json(j.to_string()));
    }
    Ok(())
}
struct FileDroper(std::path::PathBuf);
impl Drop for FileDroper {
    fn drop(&mut self) {
        tracing::info!("exhaust {}", self.0.display());
        std::fs::remove_file(&self.0).unwrap_or_default();
    }
}

#[handler]
async fn depoly(
    req: &mut Request,
    _depot: &mut Depot,
    res: &mut Response,
) -> Result<(), CatchError> {
    let file = req
        .file("file")
        .await
        .ok_or(anyhow::anyhow!("file not found in the request"))?;
    let file_saved_path = file.path().to_owned();
    let _file_droper = FileDroper(file_saved_path.clone());
    let file_size = req
        .form::<u64>("file_size")
        .await
        .ok_or(anyhow::anyhow!("file_size not found in the request"))?;
    let token = req
        .form::<&str>("token")
        .await
        .ok_or(anyhow::anyhow!("token not found in the request"))?
        .trim()
        .to_owned();
    let db = DB_CONN.get().ok_or(anyhow::anyhow!("database is busy"))?;
    let info = ProjectTb::find()
        .filter(project_tb::Column::Token.eq(token.clone()))
        .one(db)
        .await?;
    if info.is_some() {
        let received_size = file_saved_path.metadata()?.len();
        if received_size != file_size {
            let j = serde_json::json!({
                "status":100,
                "msg":format!("object size is not consistent, received:{received_size}, actual:{file_size}")
            });
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Json(j.to_string()));
        } else {
            let file = tokio::fs::read(&file_saved_path).await?;
            let file_name = file_saved_path
                .file_name()
                .ok_or(anyhow::anyhow!("file_name cannot be acquired"))?
                .to_str()
                .ok_or(anyhow::anyhow!(
                    "file_name cannot be converted to canonical string"
                ))?
                .to_owned();
            let file_part = reqwest::multipart::Part::bytes(file)
                .file_name(file_name)
                .mime_str("application/zip")?;
            let form = reqwest::multipart::Form::new().part("file", file_part);
            let info = info.unwrap();
            let parent_id = info.parent_id;
            let depoly_path = info.path;
            let group = HostTb::find()
                .filter(host_tb::Column::Id.eq(parent_id))
                .one(db)
                .await?
                .ok_or(anyhow::anyhow!("Cannot find group information"))?;
            let secret = group.secret;
            let protocol = group.protocol;
            let remote_addr = group.host;
            let form = form
                .text("file_size", file_size.to_string())
                .text("depoly_path", depoly_path);
            let client = reqwest::Client::new();
            let url = format!("{protocol}://{remote_addr}/depoly");
            let resp = client
                .post(url)
                .header("secret", secret)
                .multipart(form)
                .send()
                .await?;
            let status = resp.status();
            let json = resp.json::<serde_json::Value>().await?;
            res.status_code(status);
            res.render(Text::Json(json.to_string()));
        }
    } else {
        let j = serde_json::json!({
            "status":400,
            "msg":format!("invalid token: {token}")
        });
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Text::Json(j.to_string()));
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config: Config = FromConfigFile::from_config_file("./config.toml")?;
    let conn = Database::connect(&config.db_url).await?;
    DB_CONN.get_or_init(move || conn);

    use time::{macros::format_description, UtcOffset};
    use tracing_subscriber::fmt::time::OffsetTime;
    let local_time = OffsetTime::new(
        UtcOffset::from_hms(8, 0, 0).unwrap(),
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"),
    );
    let file_appender = tracing_appender::rolling::hourly("./logs", "service.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_timer(local_time)
        .with_max_level(tracing::Level::INFO)
        .with_writer(non_blocking)
        .init();

    let auth_handler: JwtAuth<JwtClaims> = JwtAuth::new(config.secret_key.to_owned())
        .finders(vec![
            Box::new(HeaderFinder::new()), // Box::new(CookieFinder::new("jwt_token")),
        ])
        .response_error(false);

    let cors_handler = Cors::new()
        .allow_origin("*")
        .allow_headers("authorization")
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .into_handler();
    let router = Router::with_path("api").hoop(auth_handler);
    let router = router.push(
        Router::with_path("login")
            .post(Login {
                secret_key: config.secret_key.clone(),
                admin: config.admin.clone(),
            })
            .options(handler::empty()),
    );
    let router = router.push(
        Router::with_path("host/add")
            .hoop(AuthorGuard)
            .post(add_host)
            .options(handler::empty()),
    );
    let router = router.push(
        Router::with_path("host/list")
            .hoop(AuthorGuard)
            .get(host_list)
            .options(handler::empty()),
    );
    let router = router.push(
        Router::with_path("host/del")
            .hoop(AuthorGuard)
            .post(del_host)
            .options(handler::empty()),
    );
    let router = router.push(
        Router::with_path("host/edit")
            .hoop(AuthorGuard)
            .post(edit_host)
            .options(handler::empty()),
    );
    let router = router.push(
        Router::with_path("project/add")
            .hoop(AuthorGuard)
            .post(add_project)
            .options(handler::empty()),
    );
    let router = router.push(
        Router::with_path("project/del")
            .hoop(AuthorGuard)
            .post(del_project)
            .options(handler::empty()),
    );
    let router = router.push(
        Router::with_path("project/edit")
            .hoop(AuthorGuard)
            .post(edit_project)
            .options(handler::empty()),
    );

    let root_router = Router::new()
        .hoop(cors_handler)
        .push(Router::with_path("depoly").post(depoly));
    let root_router = root_router.push(router);
    let acceptor = TcpListener::new(&config.host).bind().await;
    Server::new(acceptor).serve(root_router).await;
    Ok(())
}
