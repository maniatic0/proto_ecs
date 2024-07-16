/// Events are issued by the windowing system, usually in a platform specific manner 
/// but translated to this canonical Event data types to abstract platform-specific APIs


pub struct Event {
    handled : bool,
    event_type : Type
}

pub enum Type {
    
}