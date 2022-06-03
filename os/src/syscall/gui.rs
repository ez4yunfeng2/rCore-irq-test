use alloc::{sync::Arc, vec::{Vec, }, string::ToString};
use embedded_graphics::{prelude::{Point, Size}, primitives::arc};

use crate::{gui::{Component, Panel, ImageComp, IconController, Terminal, Button}, fs::ROOT_INODE, sync::UPSafeCell};

static DT: &[u8] = include_bytes!("../assert/desktop.bmp");

lazy_static::lazy_static!(
    pub static ref DESKTOP:UPSafeCell<Arc<dyn Component>> = unsafe {
        UPSafeCell::new(Arc::new(Panel::new(Size::new(1024, 768), Point::new(0, 0))))
    };
    pub static ref PAD:UPSafeCell<Option<Arc<Terminal>>> = unsafe {
        UPSafeCell::new(None)
    };
);

pub fn create_desktop() -> isize {
    let mut p:Arc<dyn Component + 'static> = Arc::new(Panel::new(Size::new(1024, 768), Point::new(0, 0)));
    let image = ImageComp::new(Size::new(1024, 768),Point::new(0, 0),DT,Some(p.clone()));
    let icon = IconController::new(ROOT_INODE.ls(), Some(p.clone()));
    p.add(Arc::new(image));
    p.add(Arc::new(icon));
    let mut desktop = DESKTOP.exclusive_access();
    *desktop = p;
    desktop.paint();
    drop(desktop);
    create_terminal();
    1
}

pub fn create_terminal() {
    let desktop = DESKTOP.exclusive_access();
    let arc_t = Arc::new(
        Terminal::new(
            Size::new(400, 400), 
            Point::new(200, 100), 
            Some(desktop.clone()), 
            Some("demo.txt".to_string()),
            "".to_string()
        )
    );
    let text = Panel::new(Size::new(400, 400), Point::new(200, 100));
    let button = Button::new(Size::new(20, 20), Point::new(370, 10), Some(arc_t.clone()), "X".to_string());
    arc_t.add(Arc::new(text));
    arc_t.add(Arc::new(button));
    arc_t.paint();
    desktop.add(arc_t.clone());
    let mut pad = PAD.exclusive_access();
    *pad = Some(arc_t);
}

