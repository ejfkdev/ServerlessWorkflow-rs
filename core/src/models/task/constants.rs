string_constants! {
    /// Enumerates all supported task types
    TaskType {
        CALL => "call",
        DO => "do",
        EMIT => "emit",
        FOR => "for",
        FORK => "fork",
        LISTEN => "listen",
        RAISE => "raise",
        RUN => "run",
        SET => "set",
        SWITCH => "switch",
        TRY => "try",
        WAIT => "wait",
    }
}

string_constants! {
    /// Enumerates all supported flow directive values
    FlowDirective {
        CONTINUE => "continue",
        EXIT => "exit",
        END => "end",
    }
}

string_constants! {
    /// Enumerates all supported process types
    ProcessType {
        CONTAINER => "container",
        SCRIPT => "script",
        SHELL => "shell",
        WORKFLOW => "workflow",
    }
}

string_constants! {
    /// Enumerates all supported container cleanup policies
    ContainerCleanupPolicy {
        ALWAYS => "always",
        EVENTUALLY => "eventually",
        NEVER => "never",
    }
}

string_constants! {
    /// Enumerates all supported event read modes
    EventReadMode {
        DATA => "data",
        ENVELOPE => "envelope",
        RAW => "raw",
    }
}

string_constants! {
    /// Enumerates all supported HTTP output formats
    HttpOutputFormat {
        RAW => "raw",
        CONTENT => "content",
        RESPONSE => "response",
    }
}

string_constants! {
    /// Enumerates all supported process return types
    ProcessReturnType {
        STDOUT => "stdout",
        STDERR => "stderr",
        CODE => "code",
        ALL => "all",
        NONE => "none",
    }
}

string_constants! {
    /// Enumerates all supported extension target types
    ExtensionTarget {
        CALL => "call",
        COMPOSITE => "composite",
        EMIT => "emit",
        FOR => "for",
        LISTEN => "listen",
        RAISE => "raise",
        RUN => "run",
        SET => "set",
        SWITCH => "switch",
        TRY => "try",
        WAIT => "wait",
        A2A => "a2a",
        ALL => "all",
    }
}

string_constants! {
    /// Enumerates all supported OAuth2 grant types
    OAuth2GrantType {
        AUTHORIZATION_CODE => "authorization_code",
        CLIENT_CREDENTIALS => "client_credentials",
        PASSWORD => "password",
        REFRESH_TOKEN => "refresh_token",
        TOKEN_EXCHANGE => "urn:ietf:params:oauth:grant-type:token-exchange",
    }
}
