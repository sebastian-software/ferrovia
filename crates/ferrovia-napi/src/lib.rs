use ferrovia_core::{Config, optimize as optimize_svg};
use napi::bindgen_prelude::Result;
use napi_derive::napi;

#[napi(object)]
pub struct OptimizeOptions {
    pub config_json: Option<String>,
}

#[napi(object)]
pub struct OptimizeResponse {
    pub data: String,
}

#[napi]
pub fn optimize(svg: String, options: Option<OptimizeOptions>) -> Result<OptimizeResponse> {
    let config = match options.and_then(|options| options.config_json) {
        Some(raw) => serde_json::from_str::<Config>(&raw)
            .map_err(|error| napi::Error::from_reason(error.to_string()))?,
        None => Config::default(),
    };

    let result =
        optimize_svg(&svg, &config).map_err(|error| napi::Error::from_reason(error.to_string()))?;
    Ok(OptimizeResponse { data: result.data })
}
