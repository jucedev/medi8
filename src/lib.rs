use std::{any::{Any, TypeId}, collections::HashMap};

/// Marker trait for all requests that go through the mediator
pub trait Request: 'static {}

/// Marker trait for requests that represent notifications
pub trait Notification: 'static {}

/// Processes a request and returns a response
pub trait RequestHandler<R: Request, Resp>: 'static {
    fn handle(&self, request: R) -> Resp;
}

/// Processes a notification
pub trait NotificationHandler<N: Notification>: 'static {
    fn handle(&self, notification: N);
}

/// Core mediator struct, owns and dispatches handlers
pub struct Mediator {
    /// store handlers by request type ID
    request_handlers: HashMap<TypeId, Box<dyn Any>>,
    notification_handlers: HashMap<TypeId, Vec<Box<dyn Any>>>,
}

impl Mediator {
    /// Create a new empty mediator
    pub fn new() -> Self {
        Self {
            request_handlers: HashMap::new(),
            notification_handlers: HashMap::new(),
        }
    }

    /// Register a handler for a given request type
    pub fn register_request<R, Resp, H>(&mut self, handler: H)
    where
        R: Request,
        Resp: 'static,
        H: RequestHandler<R, Resp> + 'static,
    {
        let type_id = TypeId::of::<R>();

        // Box the request handler
        let boxed_handler: Box<dyn RequestHandler<R, Resp>> = Box::new(handler);

        // Box it again as 'Any' so we can downcast it safely later
        self.request_handlers.insert(type_id, Box::new(boxed_handler));
    }

    /// Register a handler for a given notification type
    pub fn register_notification<N, H>(&mut self, handler: H)
    where
        N: Notification + 'static,
        H: NotificationHandler<N> + 'static,
    {
        let type_id = TypeId::of::<N>();

        let entry = self
            .notification_handlers
            .entry(type_id)
            .or_insert_with(Vec::new);

        // Box the notification handler
        let handler: Box<dyn NotificationHandler<N>> = Box::new(handler);

        // Box it again as 'Any' so we can downcast it safely later
        entry.push(Box::new(handler));
    }

    /// Dispatch a request to the appropriate handler, and return the response
    pub fn send_request<R, Resp>(&self, request: R) -> Result<Resp, String>
    where 
        R: Request,
        Resp: 'static,
    {
        let type_id = TypeId::of::<R>();

        // Get the boxed handler
        let boxed_handler = self
            .request_handlers
            .get(&type_id)
            .ok_or_else(|| "No handler registered for request.".to_string())?;

        let handler = boxed_handler
            .downcast_ref::<Box<dyn RequestHandler<R, Resp>>>()
            .ok_or_else(|| "Handler found, but type mismatch occurred.".to_string())?;

        Ok(handler.handle(request))
    }

    /// Publish a notification to all handlers registered for its type
    pub fn send_notification<N>(&self, notification: N)
    where 
        N: Notification + Clone,
    {
        let type_id = TypeId::of::<N>();

        // Find handlers by the notification's TypeId
        let handlers = match self.notification_handlers.get(&type_id) {
            Some(h) => h,
            None => return,
        };

        // Iterate over each of the stored boxed handlers
        for boxed_handler in handlers {
            // Attempt to downcast to the expected handler
            if let Some(handler) = boxed_handler.downcast_ref::<Box<dyn NotificationHandler<N>>>()
            {
                // Clone the notification so that each handler has its own copy
                handler.handle(notification.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

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
        mediator.register_request::<Greet, String, _>(GreetHandler);

        // Send a Greet request
        let result: Result<String, String> = mediator.send_request(Greet {
            name: "Alice".into()
        });

        assert_eq!(result.unwrap(), "Hello Alice!");
    }

    #[test]
    fn mediator_returns_error_for_unregistered_command() {
        let mediator = Mediator::new();

        // Send a Greet request without registering the handler
        let result: Result<String, _> = mediator.send_request(Greet {
            name: "Bob".into()
        });

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No handler registered for request.");
    }

    #[test]
    fn mediator_returns_error_for_handler_type_mismatch() {
        let mut mediator = Mediator::new();

        // Register the handler with the expected output (String)
        mediator.register_request::<Greet, String, _>(GreetHandler);

        // Send a Greet request with an invalid return type (usize)
        let result: Result<usize, String> = mediator.send_request(Greet {
            name: "Steve".into()
        });

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Handler found, but type mismatch occurred.");
    }

    // Notification tests

    /// A test command that implements Notification
    #[derive(Clone)]
    struct Ping;

    impl Notification for Ping {}

    /// A test handler that stores whether it has been called
    struct PingHandler {
        // using Arc<Mutex<>> to allow for shared mutation across threads
        was_called: Arc<Mutex<bool>>,
    }

    impl NotificationHandler<Ping> for PingHandler {
        fn handle(&self, _notification: Ping) {
            // Record when the handler has been called so we can track it
            let mut flag = self.was_called.lock().unwrap();
            *flag = true;
        }
    }

    #[test]
    fn mediator_invokes_registered_notification() {
        // Create a shared flag so we can assert whether the handler was called later
        let was_called = Arc::new(Mutex::new(false));

        // Create the handler using the shared flag
        let handler = PingHandler {
            was_called: Arc::clone(&was_called),
        };

        // Create and register the handler with the mediator
        let mut mediator = Mediator::new();
        mediator.register_notification(handler);

        // Publish the notification
        mediator.send_notification(Ping);

        assert_eq!(*was_called.lock().unwrap(), true);
    }

    #[test]
    fn mediator_ignores_unhandled_notification() {
        // Create the mediator but don't register any notifications
        let mediator = Mediator::new();

        // Publish a notification - it should not panic even when no handlers exist
        mediator.send_notification(Ping);
    }
}
