use config_file::FromConfigFile;
use salvo::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
struct Config {
    secret: String,
    host: String,
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
        .ok_or(anyhow::anyhow!("file not found in request"))?;
    let file_path = file
        .path()
        .to_owned()
        .to_str()
        .ok_or(anyhow::anyhow!("file saved in invalid path"))?
        .to_owned();
    let receive_size = file.size();

    let depoly_path = req
        .form::<String>("depoly_path")
        .await
        .ok_or(anyhow::anyhow!("depoly_path not found in request"))?;
    let file_size = req
        .form::<u64>("file_size")
        .await
        .ok_or(anyhow::anyhow!("file_size not found in request"))?;
    //println!("{}", line!());
    if receive_size != file_size {
        return Err(anyhow::anyhow!("file size checking cannot pass").into());
    } else {
        file_core::decompress_zip_to_dir(&file_path, &depoly_path, Some(|_| {}))?;
    };
    let j = serde_json::json!({
        "status":200,
        "msg":"The project is successfully depolied"
    });
    res.render(Text::Json(j.to_string()));
    Ok(())
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

struct ValidateReq(String);
#[handler]
impl ValidateReq {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) -> Result<(), CatchError> {
        let secret = req
            .header::<String>("secret")
            .ok_or(anyhow::anyhow!("invalid request"))?;
        if secret != self.0 {
            return Err(anyhow::anyhow!("unauthorized request").into());
        } else {
            ctrl.call_next(req, depot, res).await;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let config = Config::from_config_file("./config.toml").expect("config file not found");
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

    let router = Router::with_path("depoly")
        .hoop(ValidateReq(config.secret.clone()))
        .post(depoly);
    let acceptor = TcpListener::new(config.host.clone()).bind().await;
    Server::new(acceptor).serve(router).await;
}