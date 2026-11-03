#![no_main]

use ferry_proto::input_agent::{Request, Response};
use ferry_proto::SecureAction;
use ferry_trust::InputAuthorization;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    if let Ok(body) = <&[u8; 32]>::try_from(bytes) {
        if let Ok(action) = SecureAction::decode(body) { assert_eq!(action.encode().unwrap(), *body); }
    }
    if let Ok(code) = std::str::from_utf8(bytes) {
        if let Ok(authorization) = InputAuthorization::from_code(code) {
            assert_eq!(InputAuthorization::from_code(&authorization.to_code()).unwrap(), authorization);
        }
    }
    if let Ok(request) = Request::decode(bytes) {
        assert_eq!(Request::decode(&request.encode().unwrap()).unwrap(), request);
    }
    if let Ok(response) = Response::decode(bytes) {
        assert_eq!(Response::decode(&response.encode()).unwrap(), response);
    }
});
