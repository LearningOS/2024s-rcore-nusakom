#[derive(Debug)]
pub enum TranslateError {
    NotMapped
}

pub type TranslateResult<T> = Result<T,TranslateError>;

#[derive(Debug)]
pub enum MapError {
    NotEnoughMemory,
    Mapped
}

/// Result of mapping
pub type MapResult<T> = Result<T,MapError>;

#[derive(Debug)]
pub enum UnMapError {
    UnMapped
}

/// Result of unmapping
pub type UnMapResult<T> = Result<T,UnMapError>;