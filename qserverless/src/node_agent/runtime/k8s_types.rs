// Copyright (c) 2021 Quark Container Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::BTreeMap;

//use k8s_openapi::api::core::v1 as k8s;
use qobjs::v1alpha2::{self as cri};

pub struct ImageSpec {
    // ID of the image.
    pub image: String,
    // The annotations for the image.
    // This should be passed to CRI during image pulls and returned when images are listed.
    pub annotations: Vec<Annotation>,
}

// Image contains basic information about a container image.
pub struct Image {
    // ID of the image.
	pub id: String, 
	// Other names by which this image is known.
	pub repoTags: Vec<String>,
	// Digests by which this image is known.
	pub repoDigests: Vec<String>,
	// The size of the image in bytes.
	pub size: i64,
	// ImageSpec for the image which include annotations.
	pub spec: ImageSpec,
	// Pin for preventing garbage collection
	pub pinned: bool,
}

// EnvVar represents the environment variable.
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

// Annotation represents an annotation.
pub struct Annotation {
    pub name: String,
    pub value: String,
}

// Mount represents a volume mount.
pub struct Mount {
	// Name of the volume mount.
	// TODO(yifan): Remove this field, as this is not representing the unique name of the mount,
	// but the volume name only.
	pub name: String,
	// Path of the mount within the container.
	pub containerPath: String,
	// Path of the mount on the host.
	pub hostPath: String,
	// Whether the mount is read-only.
	pub readOnly: bool,
	// Whether the mount needs SELinux relabeling
	pub SELinuxRelabel: bool,
	// Requested propagation mode
	pub Propagation: cri::MountPropagation,
}

// Protocol defines network protocols supported for things like container ports.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ProtocolType {
    // ProtocolTCP is the TCP protocol.
	TCP, // Protocol = "TCP"
	// ProtocolUDP is the UDP protocol.
	UDP, // Protocol = "UDP"
	// ProtocolSCTP is the SCTP protocol.
	SCTP, // Protocol = "SCTP"
}

// PortMapping contains information about the port mapping.
pub struct PortMapping {
	// Protocol of the port mapping.
	pub Protocol: ProtocolType,
	// The port number within the container.
	pub containerPort: i32,
	// The port number on the host.
	pub hostPort: i32,
	// The host IP.
	pub hostIP: String,
}

// DeviceInfo contains information about the device.
pub struct DeviceInfo {
	// Path on host for mapping
	pub pathOnHost: String, 
	// Path in Container to map
	pub pathInContainer: String, 
	// Cgroup permissions
	pub permissions: String, 
}

// RunContainerOptions specify the options which are necessary for running containers
pub struct RunContainerOptions {
	// The environment variables list.
	pub envs: Vec<EnvVar>,
	// The mounts for the containers.
	pub mounts: Vec<Mount>,
	// The host devices mapped into the containers.
	pub devices: Vec<DeviceInfo>,
	// The annotations for the container
	// These annotations are generated by other components (i.e.,
	// not users). Currently, only device plugins populate the annotations.
	pub annotations: Vec<Annotation>,
	// If the container has specified the TerminationMessagePath, then
	// this directory will be used to create and mount the log file to
	// container.TerminationMessagePath
	pub podContainerDir: String,
	// The type of container rootfs
	pub readOnly: bool,
	// hostname for pod containers
	pub hostname: String,
	// EnableHostUserNamespace sets userns=host when users request host namespaces (pid, ipc, net),
	// are using non-namespaced capabilities (mknod, sys_time, sys_module), the pod contains a privileged container,
	// or using host path volumes.
	// This should only be enabled when the container runtime is performing user remapping AND if the
	// experimental behavior is desired.
	pub enableHostUserNamespace: bool,
}

// VolumeInfo contains information about the volume.
pub struct VolumeInfo {
	// Mounter is the volume's mounter

	// Mounter volume.Mounter

	// BlockVolumeMapper is the Block volume's mapper
	// BlockVolumeMapper volume.BlockVolumeMapper

	// SELinuxLabeled indicates whether this volume has had the
	// pod's SELinux label applied to it or not
	pub SELinuxLabeled: bool,
	// Whether the volume permission is set to read-only or not
	// This value is passed from volume.spec
	pub readOnly: bool,
	// Inner volume spec name, which is the PV name if used, otherwise
	// it is the same as the outer volume spec name.
	pub innerVolumeSpecName: String,
}

// VolumeMap represents the map of volumes.
pub type VolumeMap = BTreeMap<String, VolumeInfo>;

// RuntimeConditionType is the types of required runtime conditions.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RuntimeConditionType {
    // RuntimeReady means the runtime is up and ready to accept basic containers.
	RuntimeReady, // "RuntimeReady"
	// NetworkReady means the runtime network is up and ready to accept containers which require network.
	NetworkReady, // "NetworkReady"
}
 
// RuntimeCondition contains condition information for the runtime.
#[derive(Debug, Clone)]
pub struct RuntimeCondition {
	// Type of runtime condition.
	pub type_: RuntimeConditionType,
	// Status of the condition, one of true/false.
	pub status: bool,
	// Reason is brief reason for the condition's last transition.
	pub reason: String,
	// Message is human readable message indicating details about last transition.
	pub massage: String,
}

impl RuntimeCondition {
    pub fn Copy(&self) -> Self {
        return Self {
            type_: self.type_,
            status: self.status,
            reason: self.reason.clone(),
            massage: self.massage.clone(),
        }
    }
}

// RuntimeStatus contains the status of the runtime.
pub struct  RuntimeStatus {
	// Conditions is an array of current observed runtime conditions.
	pub conditions: Vec<RuntimeCondition>,
}

impl RuntimeStatus {
   // GetRuntimeCondition gets a specified runtime condition from the runtime status.
    pub fn GetRuntimeCondition(&self, t: RuntimeConditionType) -> Option<RuntimeCondition> {
        for c in &self.conditions {
            if c.type_ == t {
                return Some(c.Copy());
            }
        }
        return None
    } 
}