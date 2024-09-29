use actix_web::web;
use crate::handlers::{unit_converter, block_to_timestamp,abi_handler};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/convert")
            .route(web::post().to(unit_converter::unit_converter))
    )
    .service(
        web::resource("/block-to-timestamp")
            .route(web::post().to(block_to_timestamp::block_to_timestamp))
    )
    .service(
        web::resource("/convert-abi-to-solidity")
            .route(web::post().to(abi_handler::convert_abi_to_solidity))
    )
    .service(
        web::resource("/convert-solidity-to-abi")
            .route(web::post().to(abi_handler::convert_solidity_to_abi)),
    );
}