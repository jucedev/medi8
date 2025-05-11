use std::{any::{Any, TypeId}, collections::HashMap};

/// Marker trait for all requests that go through the mediator
pub trait Request: 'static {}

/// Processes a request and returns a response
pub trait RequestHandler<R: Request, Resp>: 'static {
    fn handle(&self, request: R) -> Resp;
}

/// Core mediator struct, owns and dispatches handlers
pub struct Mediator {
    /// store handlers by request type ID
    handlers: HashMap<TypeId, Box<dyn Any>>,
}

impl Mediator {
    /// Create a new empty mediator
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a given request type
    pub fn register<R, Resp, H>(&mut self, handler: H)
    where
        R: Request,
        Resp: 'static,
        H: RequestHandler<R, Resp> + 'static,
    {
        let type_id = TypeId::of::<R>();

        // Box the request handler
        let boxed_handler: Box<dyn RequestHandler<R, Resp>> = Box::new(handler);

        // Box it again as 'Any' so we can downcast it safely later
        self.handlers.insert(type_id, Box::new(boxed_handler));
    }

    /// Dispatch a request to the appropriate handler, and return the response
    pub fn send<R, Resp>(&self, request: R) -> Result<Resp, String>
    where 
        R: Request,
        Resp: 'static,
    {
        let type_id = TypeId::of::<R>();

        // Get the boxed handler
        let boxed_handler = self
            .handlers
            .get(&type_id)
            .ok_or_else(|| "No handler registered for request.".to_string())?;

        let handler = boxed_handler
            .downcast_ref::<Box<dyn RequestHandler<R, Resp>>>()
            .ok_or_else(|| "Handler found, but type mismatch occurred.".to_string())?;

        Ok(handler.handle(request))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test command that implements Request
    #[derive(Debug)]
    struct Greet {
        name: String,
    }

    impl Request for Greet {}

    /// A handler for the greet command
    struct GreetHandler;

    impl RequestHandler<Greet, String> for GreetHandler {
        fn handle(&self, request: Greet) -> String {
            format!("Hello {}!", request.name)
        }
    }

    #[test]
    fn mediator_handles_registered_command() {
        let mut mediator = Mediator::new();

        // Register the Greet handler
        mediator.register::<Greet, String, _>(GreetHandler);

        // Send a Greet request
        let result: Result<String, String> = mediator.send(Greet {
            name: "Alice".into()
        });

        assert_eq!(result.unwrap(), "Hello Alice!");
    }

    #[test]
    fn mediator_returns_error_for_unregistered_command() {
        let mediator = Mediator::new();

        // Send a Greet request without registering the handler
        let result: Result<String, _> = mediator.send(Greet {
            name: "Bob".into()
        });

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No handler registered for request.");
    }

    #[test]
    fn mediator_returns_error_for_handler_type_mismatch() {
        let mut mediator = Mediator::new();

        // Register the handler with the expected output (String)
        mediator.register::<Greet, String, _>(GreetHandler);

        // Send a Greet request with an invalid return type (usize)
        let result: Result<usize, String> = mediator.send(Greet {
            name: "Steve".into()
        });

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Handler found, but type mismatch occurred.");
    }
}
