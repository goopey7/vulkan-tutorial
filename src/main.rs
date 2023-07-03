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

use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_void;

use thiserror::Error;

use vulkanalia::vk::
{
	ExtDebugUtilsExtension,
	DebugUtilsMessageTypeFlagsEXT,
	DebugUtilsMessageSeverityFlagsEXT,
	KhrSurfaceExtension,
	KhrSwapchainExtension,
};


const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);
const VALIDATION_ENABLED: bool = cfg!(debug_assertions);
const VALIDATION_LAYER: vk::ExtensionName =
	vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");
const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

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
	data: AppData,
	device: Device,
}

impl App
{
	/// Creates our Vulkan app.
	unsafe fn create(window: &Window) -> Result<Self>
	{
		let loader = LibloadingLoader::new(LIBRARY)?;
		let entry = Entry::new(loader).map_err(|error| anyhow!(error))?;
		let mut data = AppData::default();
		let instance = create_instance(window, &entry, &mut data)?;
		data.surface = vk_window::create_surface(&instance, &window, &window)?;
		select_physical_device(&instance, &mut data)?;
		let device = create_logical_device(&entry, &instance, &mut data)?;
		Ok(Self {entry, instance, data, device})
	}

	/// Renders a frame for our Vulkan app.
	unsafe fn render(&mut self, window: &Window) -> Result<()>
	{
		Ok(())
	}

	/// Destroys our Vulkan app.
	unsafe fn destroy(&mut self)
	{
		self.device.destroy_device(None);
		self.instance.destroy_surface_khr(self.data.surface, None);
		if VALIDATION_ENABLED
		{
			self.instance.destroy_debug_utils_messenger_ext(self.data.messenger, None);
		}
		self.instance.destroy_instance(None);
	}
}

/// The Vulkan handles and associated properties used by our Vulkan app.
#[derive(Clone, Debug, Default)]
struct AppData
{
	messenger: vk::DebugUtilsMessengerEXT,
	physical_device: vk::PhysicalDevice,	
	graphics_queue: vk::Queue,
	surface: vk::SurfaceKHR,
	presentation_queue: vk::Queue,
}

unsafe fn create_instance(window: &Window, entry: &Entry, data: &mut AppData) -> Result<Instance>
{
	let application_info = vk::ApplicationInfo::builder()
		.application_name(b"Vulkan Tutorial (Rust)\0")
		.application_version(vk::make_version(1, 0, 0))
		.engine_name(b"No Engine\0")
		.engine_version(vk::make_version(1, 0, 0))
		.api_version(vk::make_version(1, 0, 0));

	let available_layers = entry.enumerate_instance_layer_properties()?
		.iter()
		.map(|layer| layer.layer_name)
		.collect::<HashSet<_>>();

	if VALIDATION_ENABLED && !available_layers.contains(&VALIDATION_LAYER)
	{
		return Err(anyhow!("Validation layer requested but not supported"));
	}

	let layers = if VALIDATION_ENABLED
	{
		vec![VALIDATION_LAYER.as_ptr()]
	}
	else
	{
		vec![]
	};

	let mut extensions = vk_window::get_required_instance_extensions(window)
		.iter()
		.map(|extension| extension.as_ptr())
		.collect::<Vec<_>>();

	if VALIDATION_ENABLED
	{
		extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
	}

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

	let mut info = vk::InstanceCreateInfo::builder()
		.application_info(&application_info)
		.enabled_extension_names(&extensions)
		.enabled_layer_names(&layers)
		.flags(flags);

	let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
		.message_severity(DebugUtilsMessageSeverityFlagsEXT::all())
		.message_type(DebugUtilsMessageTypeFlagsEXT::all())
		.user_callback(Some(debug_callback));

	if VALIDATION_ENABLED
	{
		info = info.push_next(&mut debug_info);
	}

	let instance = entry.create_instance(&info, None)?;

	if VALIDATION_ENABLED
	{
		let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
			.message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
			.message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
			.user_callback(Some(debug_callback));

		data.messenger = instance.create_debug_utils_messenger_ext(&debug_info, None)?;
	}

	Ok(instance)
}

#[derive(Copy, Clone, Debug)]
struct QueueFamilyIndices
{
	graphics: u32,
	presentation: u32,
}

impl QueueFamilyIndices
{
	unsafe fn get(
		instance: &Instance,
		data: &AppData,
		physical_device: vk::PhysicalDevice,
		) -> Result<Self>
	{
		let properties = instance.get_physical_device_queue_family_properties(physical_device);
		let graphics = properties
			.iter()
			.position(|properties| properties.queue_flags.contains(vk::QueueFlags::GRAPHICS))
			.map(|index| index as u32);

		let mut presentation = None;

		for(index, properties) in properties.iter().enumerate()
		{
			if instance.get_physical_device_surface_support_khr
				(
					physical_device,
					index as u32,
					data.surface
				)?
			{
				presentation = Some(index as u32);
				break;
			}
		}

		if let (Some(graphics), Some(presentation)) = (graphics, presentation)
		{
			Ok(Self {graphics, presentation})
		}
		else
		{
			Err(anyhow!(SuitabilityError("Missing required queue families")))
		}
	}
}

#[derive(Clone, Debug)]
struct SwapchainSupport
{
	capabilities: vk::SurfaceCapabilitiesKHR,
	formats: Vec<vk::SurfaceFormatKHR>,
	present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupport
{
	unsafe fn get(
		instance: &Instance,
		data: &AppData,
		physical_device: vk::PhysicalDevice,
		) -> Result<Self>
	{
		Ok(Self {
			capabilities: instance.get_physical_device_surface_capabilities_khr(
							physical_device,
							data.surface)?,
			formats: instance.get_physical_device_surface_formats_khr(
							physical_device,
							data.surface)?,

			present_modes: instance.get_physical_device_surface_present_modes_khr(
							physical_device,
							data.surface)?
		})
	}
}

#[derive(Debug, Error)]
#[error("Missing {0}")]
pub struct SuitabilityError(&'static str);

unsafe fn check_physical_device_extensions(
	instance: &Instance,
	physical_device: vk::PhysicalDevice
	) -> Result<()>
{
	let extensions = instance
		.enumerate_device_extension_properties(physical_device, None)?
		.iter()
		.map(|extension| extension.extension_name)
		.collect::<HashSet<_>>();
	if DEVICE_EXTENSIONS.iter().all(|extension| extensions.contains(extension))
	{
		Ok(())
	}
	else
	{
		Err(anyhow!(SuitabilityError("Missing required device extensions")))
	}
}

fn get_swapchain_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR
{
	formats
		.iter()
		.cloned()
		.find(|f|
			{
				f.format == vk::Format::B8G8R8A8_SRGB
							&& f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
			})
		.unwrap_or_else(|| formats[0])
}

fn get_swapchain_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR
{
	present_modes
		.iter()
		.cloned()
		.find(|mode|
			{
				*mode == vk::PresentModeKHR::MAILBOX //triple buffering
			})
		.unwrap_or(vk::PresentModeKHR::FIFO)
}

unsafe fn check_physical_device(
	instance: &Instance,
	physical_device: vk::PhysicalDevice,
	data: &AppData
	) -> Result<()>
{
	let properties = instance.get_physical_device_properties(physical_device);
	let features = instance.get_physical_device_features(physical_device);
	QueueFamilyIndices::get(instance, data, physical_device)?;

	let support = SwapchainSupport::get(instance, data, physical_device)?;
	if support.formats.is_empty() || support.present_modes.is_empty()
	{
		return Err(anyhow!(SuitabilityError("Insufficient swapchain support")));
	}
	Ok(())
}

unsafe fn select_physical_device(instance: &Instance, data: &mut AppData) -> Result<()>
{
	for physical_device in instance.enumerate_physical_devices()?
	{
		let properties = instance.get_physical_device_properties(physical_device);

		if let Err(error) = check_physical_device(instance, physical_device, data)
		{
			warn!("Skipping device ({}): {}", properties.device_name, error);
		}
		else
		{
			info!("Selected device: {}", properties.device_name);
			data.physical_device = physical_device;
			return Ok(());
		}
	}

	Err(anyhow!("No suitable physical device found"))
}

unsafe fn create_logical_device(
	entry: &Entry,
	instance: &Instance,
	data: &mut AppData,
	) -> Result<Device>
{
	let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;

	let mut unique_indices = HashSet::new();
	unique_indices.insert(indices.graphics);
	unique_indices.insert(indices.presentation);
	
	let queue_priorities = &[1.0];
	let queue_infos = unique_indices
		.iter()
		.map(|index|
			{
				vk::DeviceQueueCreateInfo::builder()
					.queue_family_index(*index)
					.queue_priorities(queue_priorities)
			}).collect::<Vec<_>>();

	let layers = if VALIDATION_ENABLED
	{
		vec![VALIDATION_LAYER.as_ptr()]
	}
	else
	{
		vec![]
	};

	let mut extensions = DEVICE_EXTENSIONS
		.iter()
		.map(|name| name.as_ptr())
		.collect::<Vec<_>>();

	// Since vulkan on macOS doesn't conform to spec
	if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION
	{
		extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
	}

	let features = vk::PhysicalDeviceFeatures::builder();

	let info = vk::DeviceCreateInfo::builder()
		.queue_create_infos(&queue_infos)
		.enabled_layer_names(&layers)
		.enabled_features(&features)
		.enabled_extension_names(&extensions);

	let device = instance.create_device(data.physical_device, &info, None)?;
	data.graphics_queue = device.get_device_queue(indices.graphics, 0);
	data.presentation_queue = device.get_device_queue(indices.presentation, 0);
	Ok(device)
}

extern "system" fn debug_callback(
	severity: vk::DebugUtilsMessageSeverityFlagsEXT,
	type_: vk::DebugUtilsMessageTypeFlagsEXT,
	data: *const vk::DebugUtilsMessengerCallbackDataEXT,
	_: *mut c_void,
	) -> vk::Bool32
{
	let data = unsafe { *data };
	let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

	if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
	{
		error!("({:?}) {}", type_, message);
	}
	else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
	{
		warn!("({:?}) {}", type_, message);
	}
	else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO
	{
		info!("({:?}) {}", type_, message);
	}
	else
	{
		trace!("({:?}) {}", type_, message);
	}

	vk::FALSE
}
