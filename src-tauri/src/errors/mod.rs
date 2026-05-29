use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("文件系统错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    DatabaseLock(String),
    #[error("参数错误: {0}")]
    InvalidInput(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
