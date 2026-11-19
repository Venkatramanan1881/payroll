//! RFC 9457 problem responses with stable codes from docs/api/ERROR_CATALOG.md.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    Unauthenticated,
    Forbidden,
    OrgSuspended,
    NotFound,
    ValidationFailed(String),
    PreconditionRequired,
    PreconditionFailed,
    PeriodLocked,
    VersionFrozen,
    IllegalTransition(String),
    IdempotencyConflict,
    EmployeeLimitReached,
    /// E18: (rule_ref, attempt_id, message shown to the employee).
    PunchRefused(String, uuid::Uuid, String),
    SelfApprovalForbidden,
    /// AC-08: (challenge token, enrolment URI on the first sign-in).
    SecondFactorRequired(String, Option<String>),
    /// A wrong authenticator code (AC-C03-05.2).
    OtpInvalid,
    /// Too many attempts (TSD §15).
    RateLimited,
    /// 409 with a catalogue code from ERROR_CATALOG.md and a detail.
    Refused(&'static str, String),
    RefusedWith(&'static str, String, serde_json::Value),
    ValidationRows(String, serde_json::Value),
    /// Phone verified for several organisations: (choice token, [(candidate_ref, display_name)]).
    OrganisationChoiceRequired(String, Vec<(String, String)>),
    Internal(anyhow::Error),
}

impl ApiError {
    fn parts(&self) -> (StatusCode, &'static str, &'static str) {
        match self {
            ApiError::Unauthenticated => (StatusCode::UNAUTHORIZED, "UNAUTHENTICATED", "Sign-in required"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", "Not allowed"),
            ApiError::OrgSuspended => (StatusCode::FORBIDDEN, "ORG_SUSPENDED", "Organisation is suspended"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND", "Not found"),
            ApiError::PreconditionFailed => (StatusCode::PRECONDITION_FAILED, "PRECONDITION_FAILED", "Resource changed since it was read"),
            ApiError::PeriodLocked => (StatusCode::CONFLICT, "PERIOD_LOCKED", "Date is in a locked period"),
            ApiError::VersionFrozen => (StatusCode::CONFLICT, "VERSION_FROZEN", "A published version cannot be changed"),
            ApiError::IllegalTransition(_) => (StatusCode::CONFLICT, "ILLEGAL_TRANSITION", "Not allowed in the current state"),
            ApiError::PreconditionRequired => (StatusCode::PRECONDITION_REQUIRED, "PRECONDITION_REQUIRED", "Required header missing"),
            ApiError::IdempotencyConflict => (StatusCode::CONFLICT, "IDEMPOTENCY_CONFLICT", "Idempotency-Key reused with a different body"),
            ApiError::EmployeeLimitReached => (StatusCode::CONFLICT, "EMPLOYEE_LIMIT_REACHED", "Employee limit reached"),
            ApiError::PunchRefused(..) => (StatusCode::UNPROCESSABLE_ENTITY, "PUNCH_REFUSED", "Punch refused"),
            ApiError::Refused(code, _) | ApiError::RefusedWith(code, ..) => (StatusCode::CONFLICT, *code, "Not allowed by the organisation's rules"),
            ApiError::SelfApprovalForbidden => (StatusCode::FORBIDDEN, "SELF_APPROVAL_FORBIDDEN", "You cannot decide your own request"),
            ApiError::SecondFactorRequired(..) => (StatusCode::UNAUTHORIZED, "SECOND_FACTOR_REQUIRED", "Enter the code from your authenticator app"),
            ApiError::OtpInvalid => (StatusCode::BAD_REQUEST, "OTP_INVALID", "The code is not correct"),
            ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED", "Too many attempts; try again later"),
            ApiError::ValidationFailed(_) | ApiError::ValidationRows(..) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_FAILED", "Request is not valid"),
            ApiError::OrganisationChoiceRequired(..) => (StatusCode::CONFLICT, "ORGANISATION_CHOICE_REQUIRED", "Choose an organisation"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL", "Internal error"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, title) = self.parts();
        if let ApiError::Internal(e) = &self {
            eprintln!("internal error: {e:#}"); // logged, never returned to the caller
        }
        let mut body = json!({
            "type": format!("https://errors.payroll.example/{code}"),
            "title": title,
            "status": status.as_u16(),
            "code": code,
            "correlation_id": uuid::Uuid::new_v4(),
        });
        match self {
            ApiError::ValidationFailed(detail) | ApiError::IllegalTransition(detail) | ApiError::Refused(_, detail) => body["detail"] = json!(detail),
            ApiError::ValidationRows(detail, rows) => {
                body["detail"] = json!(detail);
                body["errors"] = rows;
            }
            ApiError::RefusedWith(_, detail, items) => {
                body["detail"] = json!(detail);
                body["errors"] = items;
            }
            ApiError::PunchRefused(rule_ref, attempt_id, message) => {
                body["rule_ref"] = json!(rule_ref);
                body["attempt_id"] = json!(attempt_id);
                body["detail"] = json!(message);
            }
            ApiError::SecondFactorRequired(challenge, uri) => {
                body["challenge_token"] = json!(challenge);
                if let Some(u) = uri {
                    body["otpauth_uri"] = json!(u); // first sign-in: enrol this secret in the authenticator app
                }
            }
            ApiError::OrganisationChoiceRequired(choice_token, cands) => {
                body["choice_token"] = json!(choice_token);
                body["candidates"] = cands.iter().map(|(r, n)| json!({ "candidate_ref": r, "display_name": n })).collect();
            }
            _ => {}
        }
        (status, [(header::CONTENT_TYPE, "application/problem+json")], body.to_string()).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Internal(e.into())
    }
}

impl From<domain_iam::authz::AuthzError> for ApiError {
    fn from(e: domain_iam::authz::AuthzError) -> Self {
        match e {
            domain_iam::authz::AuthzError::Forbidden => ApiError::Forbidden,
            domain_iam::authz::AuthzError::Db(e) => e.into(),
        }
    }
}

impl From<domain_workforce::employee::WorkforceError> for ApiError {
    fn from(e: domain_workforce::employee::WorkforceError) -> Self {
        use domain_workforce::employee::WorkforceError as W;
        match e {
            W::Validation(m) => ApiError::ValidationFailed(m),
            W::LimitReached => ApiError::EmployeeLimitReached,
            W::PeriodLocked => ApiError::PeriodLocked,
            W::Db(e) => e.into(),
        }
    }
}

impl From<db::DomainError> for ApiError {
    fn from(e: db::DomainError) -> Self {
        use db::DomainError as D;
        match e {
            D::Validation(m) => ApiError::ValidationFailed(m),
            D::NotFound => ApiError::NotFound,
            D::IllegalTransition(m) => ApiError::IllegalTransition(m),
            D::VersionFrozen => ApiError::VersionFrozen,
            D::PeriodLocked => ApiError::PeriodLocked,
            D::IdempotencyConflict => ApiError::IdempotencyConflict,
            D::Refused(code, detail) => ApiError::Refused(code, detail),
            D::RefusedWith(code, detail, items) => ApiError::RefusedWith(code, detail, items),
            D::ValidationRows(detail, rows) => ApiError::ValidationRows(detail, rows),
            D::SelfApproval => ApiError::SelfApprovalForbidden,
            D::Forbidden => ApiError::Forbidden,
            D::Db(e) => e.into(),
        }
    }
}
