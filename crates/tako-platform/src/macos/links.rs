//! Links `tako://` (EXT-005): o macOS entrega a URL por Apple Event (kAEGetURL), não por argumento, e o winit não
//! repassa esse evento. O handler fica no `NSAppleEventManager`, que não depende do delegate do app.
//! Ele só sobrevive porque o delegate do winit 0.30 não implementa `application:openURLs:`: se um winit novo
//! implementar, o AppKit troca o handler de GURL pelo dele e os links param. Conferir ao atualizar o winit.

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventManager};

/// 'GURL': classe e ID do evento de abrir URL (kInternetEventClass / kAEGetURL).
const GURL: u32 = u32::from_be_bytes(*b"GURL");
/// '----': o parâmetro direto do evento, que traz a URL (keyDirectObject).
const DIRECT: u32 = u32::from_be_bytes(*b"----");

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "TakoUrlHandler"]
    #[ivars = Box<dyn Fn(String)>]
    struct UrlHandler;

    impl UrlHandler {
        #[unsafe(method(handleGetURL:withReplyEvent:))]
        fn handle(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            // SAFETY: `paramDescriptorForKeyword:` recebe um FourCharCode e devolve um descritor ou nil.
            let param: Option<Retained<NSAppleEventDescriptor>> =
                unsafe { msg_send![event, paramDescriptorForKeyword: DIRECT] };
            match param.and_then(|p| p.stringValue()) {
                Some(url) => (self.ivars())(url.to_string()),
                None => tracing::warn!("evento de abrir URL sem URL"),
            }
        }
    }
);

/// Chama `f` com cada URL aberta pelo sistema. Chamar antes do laço de eventos, para pegar também o link que
/// abriu o app; `f` roda na thread principal, dentro do laço.
pub fn on_open_url(mtm: MainThreadMarker, f: impl Fn(String) + 'static) {
    let handler = mtm
        .alloc::<UrlHandler>()
        .set_ivars(Box::new(f) as Box<dyn Fn(String)>);
    // SAFETY: `init` de NSObject numa instância recém-alocada com os ivars já postos.
    let handler: Retained<UrlHandler> = unsafe { msg_send![super(handler), init] };
    // SAFETY: o seletor existe em `UrlHandler` com a assinatura que o NSAppleEventManager espera
    // (`handleGetURL:withReplyEvent:`); classe e ID são FourCharCodes.
    unsafe {
        let _: () = msg_send![
            &NSAppleEventManager::sharedAppleEventManager(),
            setEventHandler: &*handler,
            andSelector: sel!(handleGetURL:withReplyEvent:),
            forEventClass: GURL,
            andEventID: GURL
        ];
    }
    // O NSAppleEventManager não retém o handler: ele vive até o processo sair.
    std::mem::forget(handler);
}
