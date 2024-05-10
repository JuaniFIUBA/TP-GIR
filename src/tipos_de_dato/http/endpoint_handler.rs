use std::{collections::HashMap, sync::Arc};

use crate::tipos_de_dato::logger::Logger;

use super::{error::ErrorHttp, request::Request, response::Response};

pub type EndpointHandler =
    fn(Request, HashMap<String, String>, Arc<Logger>) -> Result<Response, ErrorHttp>;
