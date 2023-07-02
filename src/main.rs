#![allow(
	dead_code,
	unused_variables,
	clippy::too_many_arguments,
	clippy::unnecessary_wraps
)]

use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use anyhow::{anyhow, Result};
use log::*;
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::window as vk_window;
use vulkanalia::prelude::v1_0::*;
use vulkanalia::Version;

const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

fn main() -> Result<()>
{
	pretty_env_logger::init();

	// Window

	let event_loop = EventLoop::new();
	let window = WindowBuilder::new()
		.with_title("Vulkan Tutorial (Rust)")
		.with_inner_size(LogicalSize::new(1024, 768))
		.build(&event_loop)?;

	// App

	let mut app = unsafe { App::create(&window)? };
	let mut destroying = false;
	event_loop.run(move |event, _, control_flow|
	{
		*control_flow = ControlFlow::Poll;
		match event
		{
			// Render a frame if our Vulkan app is not being destroyed.
			Event::MainEventsCleared if !destroying =>
			{
				unsafe { app.render(&window) }.unwrap()
			},
			// Destroy our Vulkan app.
			Event::WindowEvent { event: WindowEvent::CloseRequested, .. } =>
			{
				destroying = true;
				*control_flow = ControlFlow::Exit;
				unsafe { app.destroy(); }
			}
			_ => {}
		}
	});
}

/// Our Vulkan app.
#[derive(Clone, Debug)]
struct App
{
	entry: Entry,
	instance: Instance,
}

impl App
{
	/// Creates our Vulkan app.
	unsafe fn create(window: &Window) -> Result<Self>
	{
		let loader = LibloadingLoader::new(LIBRARY)?;
		let entry = Entry::new(loader).map_err(|error| anyhow!(error))?;
		let instance = create_instance(window, &entry)?;
		Ok(Self {entry, instance})
	}

	/// Renders a frame for our Vulkan app.
	unsafe fn render(&mut self, window: &Window) -> Result<()>
	{
		Ok(())
	}

	/// Destroys our Vulkan app.
	unsafe fn destroy(&mut self)
	{
		self.instance.destroy_instance(None);
	}
}

/// The Vulkan handles and associated properties used by our Vulkan app.
#[derive(Clone, Debug, Default)]
struct AppData {}

unsafe fn create_instance(window: &Window, entry: &Entry) -> Result<Instance>
{
	let application_info = vk::ApplicationInfo::builder()
		.application_name(b"Vulkan Tutorial (Rust)\0")
		.application_version(vk::make_version(1, 0, 0))
		.engine_name(b"No Engine\0")
		.engine_version(vk::make_version(1, 0, 0))
		.api_version(vk::make_version(1, 0, 0));

	let mut extensions = vk_window::get_required_instance_extensions(window)
		.iter()
		.map(|extension| extension.as_ptr())
		.collect::<Vec<_>>();

	// Since vulkan on macOS doesn't conform to spec
	// we gotta enable some additional extensions
	// if the vulkan sdk version is 1.3.216 or higher
	let flags = if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION
				{
					info!("Enabling macOS portability extensions");
					extensions.push(vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION.name.as_ptr());
					extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
					vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
				}
				else
				{
					vk::InstanceCreateFlags::empty()
				};

	let info = vk::InstanceCreateInfo::builder()
		.application_info(&application_info)
		.enabled_extension_names(&extensions)
		.flags(flags);

	Ok(entry.create_instance(&info, None)?)
}
