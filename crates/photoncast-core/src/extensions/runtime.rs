use abi_stable::std_types::RBox;
use photoncast_extension_api::{ExtensionRuntime, ExtensionRuntimeTrait};

#[derive(Clone, Default)]
pub struct ExtensionRuntimeSpawner;

impl ExtensionRuntimeSpawner {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn spawn(&self, future: photoncast_extension_api::ExtensionFuture_TO<'static, RBox<()>>) {
        tokio::spawn(async move {
            future.poll();
        });
    }
}

#[derive(Clone, Default)]
pub struct ExtensionRuntimeImpl {
    spawner: ExtensionRuntimeSpawner,
}

impl ExtensionRuntimeImpl {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spawner: ExtensionRuntimeSpawner::new(),
        }
    }

    #[must_use]
    pub fn api_handle(&self) -> ExtensionRuntime {
        ExtensionRuntime::new(self.clone())
    }
}

impl ExtensionRuntimeTrait for ExtensionRuntimeImpl {
    fn spawn(&self, future: photoncast_extension_api::ExtensionFuture_TO<'static, RBox<()>>) {
        self.spawner.spawn(future);
    }
}
