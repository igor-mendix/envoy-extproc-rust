// src/main.rs

// === Generated modules ===
// (Your existing module inclusions for google, udpa, xds, validate, envoy)
// Ensure these paths correctly point to your generated files in OUT_DIR
pub mod google {
    pub mod protobuf {
        include!(concat!(env!("OUT_DIR"), "/google.protobuf.rs"));
    }
}

pub mod udpa {
    pub mod annotations {
        include!(concat!(env!("OUT_DIR"), "/udpa.annotations.rs"));
    }
}

pub mod xds {
    pub mod annotations {
        pub mod v3 {
            include!(concat!(env!("OUT_DIR"), "/xds.annotations.v3.rs"));
        }
    }
    pub mod core {
        pub mod v3 {
            include!(concat!(env!("OUT_DIR"), "/xds.core.v3.rs"));
        }
    }
}

pub mod validate {
    include!(concat!(env!("OUT_DIR"), "/validate.rs"));
}

pub mod envoy {
    pub mod config {
        pub mod core {
            pub mod v3 {
                include!(concat!(env!("OUT_DIR"), "/envoy.config.core.v3.rs"));
            }
        }
    }
    pub mod extensions {
        pub mod filters {
            pub mod http {
                pub mod ext_proc {
                    pub mod v3 {
                        include!(concat!(
                            env!("OUT_DIR"),
                            "/envoy.extensions.filters.http.ext_proc.v3.rs"
                        ));
                    }
                }
            }
        }
    }
    pub mod r#type { // r# to use 'type' as identifier
        pub mod v3 {
            include!(concat!(env!("OUT_DIR"), "/envoy.r#type.v3.rs"));
        }
    }
    pub mod service {
        pub mod ext_proc {
            pub mod v3 {
                include!(concat!(
                    env!("OUT_DIR"),
                    "/envoy.service.ext_proc.v3.rs"
                ));
                pub const FILE_DESCRIPTOR_SET: &[u8] =
                    include_bytes!(concat!(env!("OUT_DIR"), "/ext_proc_descriptor.bin"));
            }
        }
    }
}


// === Application code ===

use envoy::service::ext_proc::v3::{
    external_processor_server::{ExternalProcessor, ExternalProcessorServer},
    processing_request, processing_response, CommonResponse, HeadersResponse, BodyResponse,
    ProcessingRequest, ProcessingResponse, TrailersResponse,
    common_response::ResponseStatus, HeaderMutation,
};
use envoy::config::core::v3::{HeaderValue, HeaderValueOption};
use envoy::r#type::v3::HttpStatus;
use envoy::r#type::v3::StatusCode as EnvoyStatusCode;

use futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Request, Response, Status, Streaming};

// Tracing related imports
use tracing::{error, info, warn, instrument, Level, debug};
use tracing_subscriber::{
    fmt,
    EnvFilter,
    // Layer and reload are no longer needed for this simpler setup
    prelude::*, // For .with() and .init()
};

use tonic_reflection::server::Builder as ReflectionBuilder;

#[derive(Debug, Default)]
pub struct MyExtProcessor {}

// Helper to get a string representation of the request type for logging
fn request_type_as_string(request_opt: &Option<processing_request::Request>) -> &'static str {
    match request_opt {
        Some(processing_request::Request::RequestHeaders(_)) => "RequestHeaders",
        Some(processing_request::Request::RequestBody(_)) => "RequestBody",
        Some(processing_request::Request::RequestTrailers(_)) => "RequestTrailers",
        Some(processing_request::Request::ResponseHeaders(_)) => "ResponseHeaders",
        Some(processing_request::Request::ResponseBody(_)) => "ResponseBody",
        Some(processing_request::Request::ResponseTrailers(_)) => "ResponseTrailers",
        None => "EmptyRequest",
    }
}

#[tonic::async_trait]
impl ExternalProcessor for MyExtProcessor {
    type ProcessStream =
        Pin<Box<dyn Stream<Item = Result<ProcessingResponse, Status>> + Send + Sync + 'static>>;

    #[instrument(name="ext_proc_stream", skip_all, fields(peer_addr))]
    async fn process(
        &self,
        request: Request<Streaming<ProcessingRequest>>,
    ) -> Result<Response<Self::ProcessStream>, Status> {
        let peer_addr = request.remote_addr().map_or_else(|| "unknown".to_string(), |a| a.to_string());
        tracing::Span::current().record("peer_addr", &tracing::field::display(&peer_addr));

        info!("New 'process' stream initiated from {}", peer_addr);
        let mut inbound_stream = request.into_inner();
        
        let (tx, rx) = mpsc::channel::<Result<ProcessingResponse, Status>>(10);

        tokio::spawn(async move {
            debug!("Spawned tokio task to handle inbound gRPC stream from Envoy.");
            
            while let Some(result) = inbound_stream.next().await {
                match result {
                    Ok(req) => {
                        let req_type_str = request_type_as_string(&req.request);
                        debug!(request_phase = %req_type_str, "Received request from Envoy");

                        let response_to_envoy = match req.request {
                            Some(processing_request::Request::RequestHeaders(headers_req)) => {
                                debug!(headers = ?headers_req.headers.as_ref().map(|h| &h.headers), end_of_stream = headers_req.end_of_stream, "Processing RequestHeaders");
                                let mut header_mutations = HeaderMutation::default();
                                let header_key = "x-processed-by-rust-request".to_string();
                                let header_val_str = "my-ext-processor".to_string();
                                
                                header_mutations.set_headers.push(HeaderValueOption {
                                    header: Some(HeaderValue {
                                        key: header_key,
                                        value: header_val_str.clone(), 
                                        raw_value: header_val_str.clone().into_bytes(),
                                    }),
                                    append: None, 
                                    append_action: envoy::config::core::v3::header_value_option::HeaderAppendAction::AppendIfExistsOrAdd.into(),
                                    keep_empty_value: false,
                                });

                                ProcessingResponse {
                                    response: Some(processing_response::Response::RequestHeaders(
                                        HeadersResponse {
                                            response: Some(CommonResponse {
                                                status: ResponseStatus::Continue.into(),
                                                header_mutation: Some(header_mutations),
                                                ..Default::default()
                                            }),
                                        },
                                    )),
                                    ..Default::default()
                                }
                            }
                            Some(processing_request::Request::RequestBody(body_req)) => {
                                debug!(bytes_received = body_req.body.len(), end_of_stream = body_req.end_of_stream, "Processing RequestBody");
                                ProcessingResponse {
                                    response: Some(processing_response::Response::RequestBody(
                                        BodyResponse {
                                            response: Some(CommonResponse {
                                                status: ResponseStatus::Continue.into(),
                                                ..Default::default()
                                            }),
                                        },
                                    )),
                                    ..Default::default()
                                }
                            }
                            Some(processing_request::Request::RequestTrailers(_trailers_req)) => {
                                debug!(trailers = ?_trailers_req.trailers.as_ref().map(|h| &h.headers), "Processing RequestTrailers");
                                let trailer_mutations = HeaderMutation {
                                     set_headers: vec![HeaderValueOption {
                                         header: Some(HeaderValue {
                                             key: "x-rust-request-trailer".to_string(),
                                             value: "added-by-processor".to_string(),
                                             raw_value: "added-by-processor".to_string().into_bytes(),
                                         }),
                                         append: None,
                                         append_action: envoy::config::core::v3::header_value_option::HeaderAppendAction::AppendIfExistsOrAdd.into(),
                                         keep_empty_value: false,
                                     }],
                                    ..Default::default()
                                };
                                ProcessingResponse {
                                    response: Some(processing_response::Response::RequestTrailers(
                                        TrailersResponse { 
                                            header_mutation: Some(trailer_mutations), 
                                        },
                                    )),
                                    ..Default::default()
                                }
                            }
                            Some(processing_request::Request::ResponseHeaders(headers_req)) => {
                                debug!(headers = ?headers_req.headers.as_ref().map(|h| &h.headers), end_of_stream = headers_req.end_of_stream, "Processing ResponseHeaders");
                                let mut header_mutations = HeaderMutation::default();
                                let header_key = "x-processed-by-rust-response".to_string();
                                let header_val_str = "my-ext-processor".to_string();
                                header_mutations.set_headers.push(HeaderValueOption {
                                    header: Some(HeaderValue {
                                        key: header_key,
                                        value: header_val_str.clone(), 
                                        raw_value: header_val_str.clone().into_bytes(),
                                    }),
                                    append: None, 
                                    append_action: envoy::config::core::v3::header_value_option::HeaderAppendAction::AppendIfExistsOrAdd.into(),
                                    keep_empty_value: false,
                                });
                                ProcessingResponse {
                                    response: Some(processing_response::Response::ResponseHeaders(
                                        HeadersResponse {
                                            response: Some(CommonResponse {
                                                status: ResponseStatus::Continue.into(),
                                                header_mutation: Some(header_mutations),
                                                ..Default::default()
                                            }),
                                        },
                                    )),
                                    ..Default::default()
                                }
                            }
                            Some(processing_request::Request::ResponseBody(body_req)) => {
                                debug!(bytes_received = body_req.body.len(), end_of_stream = body_req.end_of_stream, "Processing ResponseBody");
                                ProcessingResponse {
                                    response: Some(processing_response::Response::ResponseBody(
                                        BodyResponse {
                                            response: Some(CommonResponse {
                                                status: ResponseStatus::Continue.into(),
                                                ..Default::default()
                                            }),
                                        },
                                    )),
                                    ..Default::default()
                                }
                            }
                            Some(processing_request::Request::ResponseTrailers(_trailers_req)) => {
                                debug!(trailers = ?_trailers_req.trailers.as_ref().map(|h| &h.headers), "Processing ResponseTrailers");
                                let trailer_mutations = HeaderMutation {
                                     set_headers: vec![HeaderValueOption {
                                         header: Some(HeaderValue {
                                             key: "x-rust-response-trailer".to_string(),
                                             value: "added-by-processor".to_string(),
                                             raw_value: "added-by-processor".to_string().into_bytes(),
                                         }),
                                         append: None,
                                         append_action: envoy::config::core::v3::header_value_option::HeaderAppendAction::AppendIfExistsOrAdd.into(),
                                         keep_empty_value: false,
                                     }],
                                    ..Default::default()
                                };
                                ProcessingResponse {
                                    response: Some(processing_response::Response::ResponseTrailers(
                                        TrailersResponse { 
                                            header_mutation: Some(trailer_mutations), 
                                        },
                                    )),
                                    ..Default::default()
                                }
                            }
                            None => {
                                warn!("Received ProcessingRequest with no specific request type (request field was None).");
                                ProcessingResponse {
                                     response: Some(processing_response::Response::ImmediateResponse(
                                        envoy::service::ext_proc::v3::ImmediateResponse {
                                            status: Some(HttpStatus { code: EnvoyStatusCode::InternalServerError as i32 }),
                                            headers: None,
                                            body: "Empty request type received".to_string().into_bytes(), 
                                            grpc_status: None,
                                            details: "Processor received an empty request type.".to_string(),
                                        }
                                    )),
                                    ..Default::default()
                                }
                            }
                        };

                        if tx.send(Ok(response_to_envoy)).await.is_err() {
                            error!("gRPC receiver dropped, failed to send response to Envoy. Ending stream.");
                            break; 
                        }
                    }
                    Err(e) => {
                        error!(error = ?e, "Error receiving message from Envoy gRPC stream");
                        if tx.send(Err(e)).await.is_err() {
                            error!("gRPC receiver dropped, also failed to send error to Envoy.");
                        }
                        break; 
                    }
                }
            }
            debug!("Finished processing Envoy gRPC stream. Closing response channel.");
        });

        let response_stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(response_stream) as Self::ProcessStream))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber based on RUST_LOG env var or a default.
    let default_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,my_ext_proc_server=info")); // Default to info for self

    tracing_subscriber::registry()
        .with(default_filter)    // Use the EnvFilter directly
        .with(fmt::layer())      // Add the formatting layer (writes to stdout)
        .init();                 // Initialize this as the global tracing subscriber

    // --- gRPC Server Setup ---
    let grpc_addr = "0.0.0.0:50051".parse()?;
    let ext_proc_svc = MyExtProcessor::default();
    info!(address = %grpc_addr, "ExternalProcessorServer (gRPC) listening");

    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(envoy::service::ext_proc::v3::FILE_DESCRIPTOR_SET)
        .build_v1()?; 

    let grpc_server = Server::builder()
        .trace_fn(|_req| tracing::info_span!("grpc_request"))
        .max_concurrent_streams(Some(10000))  // default is 200
        .add_service(ExternalProcessorServer::new(ext_proc_svc))
        .add_service(reflection_service)
        .serve(grpc_addr);
    
    // Only run the gRPC server
    grpc_server.await?;

    info!("ExternalProcessorServer shut down."); // This line might not be reached if server runs indefinitely
    Ok(())
}

