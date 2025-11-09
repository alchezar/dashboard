# Proxmox VE Testbed Setup

This document outlines the steps required to set up a local Proxmox VE instance to act as the hypervisor for the dashboard project. The following instructions are for setting up Proxmox VE within a Hyper-V virtual machine on Windows.

## 1. Proxmox VE installation.

This section covers the initial download and installation of the Proxmox VE operating system.

### 1.1 Download the Proxmox VE ISO

* Navigate to the official Proxmox VE [download page](https://www.proxmox.com/en/downloads)
* Select [Proxmox Virtual Environment](https://www.proxmox.com/en/downloads/proxmox-virtual-environment)
* Download the latest **ISO Image**.

### 1.2 Create a Hyper-V Virtual Machine

A dedicated virtual machine is required to host the Proxmox VE hypervisor.

* Open **Hyper-V Manager**
* Select `Action > New > Virtual Machine`
* Specify Name and Location: Give your VM a descriptive name (e.g., Proxmox-Testbed)
* Assign at least 4096 MB (4 GB) of startup memory
* Connect the virtual switch to a network that can access the internet and is reachable from your host machine
* Create a new virtual hard disk. A size of 64 GB or more is recommended
* Select Install an operating system from a bootable image file. Browse to and select the Proxmox VE ISO file you downloaded earlier

### 1.3 Enable Nested Virtualization

Before starting the VM, you must enable nested virtualization. This allows Proxmox to run inside Hyper-V.

* Open PowerShell as an Administrator
* Run the following command, replacing `"Proxmox-Testbed"` with the name of your VM

```shell
 Set-VMProcessor -VMName "Proxmox-Testbed" -ExposeVirtualizationExtensions $true
```

### 1.4 Install Proxmox VE

* Start the virtual machine you created. It will boot from the Proxmox VE ISO
* Follow the on-screen instructions in the installer
* During the **Configuration** step, you will need to set a static IP address for the management interface (e.g., `192.168.1.121/24`). Make sure this IP is on the same subnet as your host machine and is not already in use
* Complete the installation and reboot the VM when prompted

### 1.5 Verify the Installation

* Once the VM has rebooted, open a web browser on your host machine
* Navigate to the Proxmox web UI using the static IP you configured: `https://<your-proxmox-ip>:8006`
* You will see a browser warning about an invalid SSL certificate. This is normal. Proceed to the page
* Log in with the username `root` and the password you set during installation

> Note on Logging In: When you log into the Proxmox web UI, you must select the correct Realm. For the root user, the realm is Linux PAM standard authentication (pam). For the dashboard-svc user that we will create later, the realm will be Proxmox VE authentication server (pve).

If you can see the Proxmox dashboard, the installation is successful and the testbed is ready for the next stage of configuration.

***

## 2. VM Template Creation

The API requires cloneable VM templates with **Cloud-Init** support to provision new servers. While Proxmox has a web UI, it does not allow you to directly create templates from downloaded cloud image files (like `.qcow2`). For this reason, the entire setup process must be performed using command-line tools in the Proxmox VE shell. You can access it by selecting your node in the web UI and clicking the `>_ Shell` button.

The process is similar for all operating systems: download the cloud image, create a VM, import the disk, configure the VM, and convert it into a template.

### 2.1 Ubuntu 22.04 (VMID 9000)

Download the official Ubuntu 22.04 cloud image:

```shell
wget https://cloud-images.ubuntu.com/jammy/current/jammy-server-cloudimg-amd64.img
```

Create and configure the VM template. The following commands will create the VM, import the disk, attach it, and configure Cloud-Init.

These commands assume your primary storage for disk images is named `local-lvm`. If yours has a different name, replace `local-lvm`
accordingly.

```shell
# Create a new VM with VMID 9000
qm create 9000 --name "ubuntu-2204-template" --memory 2048 --cores 2 --net0 virtio,bridge=vmbr0

# Import the downloaded disk image to storage
qm importdisk 9000 jammy-server-cloudimg-amd64.img local-lvm

# Attach the imported disk to the VM
qm set 9000 --scsihw virtio-scsi-pci --scsi0 local-lvm:vm-9000-disk-0
qm set 9000 --ide2 local-lvm:cloudinit
qm set 9000 --boot c --bootdisk scsi0

# Convert the VM into a template
qm template 9000
```

### 2.2 Debian 11 (VMID 9001)

Download the official Debian 11 cloud image:

```shell
wget https://cloud.debian.org/images/cloud/bullseye/latest/debian-11-genericcloud-amd64.qcow2

```

Create and configure the VM template:

```shell
# Create a new VM with VMID 9001
qm create 9001 --name "debian-11-template" --memory 2048 --cores 2 --net0 virtio,bridge=vmbr0

# Import the downloaded disk image
qm importdisk 9001 debian-11-genericcloud-amd64.qcow2 local-lvm

# Attach the disk and configure the VM
qm set 9001 --scsihw virtio-scsi-pci --scsi0 local-lvm:vm-9001-disk-0
qm set 9001 --ide2 local-lvm:cloudinit
qm set 9001 --boot c --bootdisk scsi0

# Convert the VM into a template
qm template 9001
```

### 2.3 CentOS 9 Stream (VMID 9002)

Download the official CentOS 9 Stream cloud image:

```shell
wget https://cloud.centos.org/centos/9-stream/x86_64/images/CentOS-Stream-GenericCloud-9-latest.x86_64.qcow2
   ```

Create and configure the VM template:

```shell
# Create a new VM with VMID 9002
qm create 9002 --name "centos-9-template" --memory 2048 --cores 2 --net0 virtio,bridge=vmbr0

# Import the downloaded disk image
qm importdisk 9002 CentOS-Stream-GenericCloud-9-latest.x86_64.qcow2 local-lvm

# Attach the disk and configure the VM
qm set 9002 --scsihw virtio-scsi-pci --scsi0 local-lvm:vm-9002-disk-0
qm set 9002 --ide2 local-lvm:cloudinit
qm set 9002 --boot c --bootdisk scsi0

# Convert the VM into a template
qm template 9002
```

After completing these steps, you will have three ready-to-use templates in your Proxmox node, which the Rust API can use to provision new servers.

***

## 3. API Access Configuration

To allow the Rust API service to securely interact with Proxmox, we need to create a dedicated user, a custom role with limited permissions, and an API token.

For a detailed overview of the Proxmox API, refer to the official documentation:

* Proxmox [API](https://pve.proxmox.com/pve-docs/api-viewer/index.html)
* Proxmox [Wiki](https://pve.proxmox.com/wiki/Proxmox_VE_API)

The following steps are performed in the Proxmox web UI and will guide you through creating a user with specific, limited permissions.

### 3.1 Create a Custom Role

First, we will create a new role that has only the minimum required permissions for the service to function.

* Navigate to `Datacenter > Permissions > Roles > Create`
* Enter the Role Name (e.g., `DashboardRole`)
* Grant the following privileges to this role:

```
Datastore.AllocateSpace, Sys.Audit, VM.Allocate, VM.Audit, VM.Clone, VM.Config.CDROM, VM.Config.Cloudinit, VM.Monitor, VM.PowerMgmt`
```

### 3.2 Create a Dedicated User

Next, create a new user that will be assigned the custom role.

* Navigate to `Datacenter > Permissions > Users` and click **Add**
* User name: `dashboard-svc`
* Realm: Proxmox VE authentication server

### 3.3 Grant Permissions to the User

Now, assign the `DashboardRole` to the `dashboard-svc` user at the root level.

* Navigate to `Datacenter > Permissions > Add > User Permission`
* Path: `/`
* User: `dashboard-svc@pve`
* Role: `DashboardRole`

> **Important:** You must set the path to `/` (root).  
> In Proxmox, a user needs two types of permission: **permission for an action** (like `VM.Clone`, which is in the Role) and **access to the item** you want to perform the action on (which is set by the Path).  
> If you don't grant access to the root path `/`, the user won't be able to "see" the VM templates to clone them, even if the role allows cloning. Setting the path to `/q ensures the user can access all necessary items.

### 3.4: Create an API Token

Finally, create an API token for the user, which the application will use to authenticate.

* Navigate to `Datacenter > Permissions > API Tokens`
* Select the user `dashboard-svc@pve` from the list and click **Add**
* Enter a descriptive name for the **Token ID** , (e.g., `dashboard_token`)
* Uncheck `Privilege Separation` box

A window will appear displaying the Token ID and the Secret.

> **Warning:** The secret value is shown only once. Copy it immediately and store it securely. You will need it for the next step.

### 3.5 Configure Environment Variables

Add the Proxmox access credentials to your project's `.env` file. The application uses these variables to connect to the Proxmox API. Replace `<proxmox-ip>`, `<user>`, `<realm>`, `<token-id>`, and `<secret>` with the actual values from your setup.

```dotenv
# URL to the Proxmox API endpoint
APP__PROXMOX__URL=https://<proxmox-ip>:8006/api2/json
# Proxmox API token for authentication
APP__PROXMOX__AUTH_HEADER=PVEAPIToken=<user>@<realm>!<token-id>=<secret>
```

Example:

```dotenv
APP__PROXMOX__URL=https://192.168.1.100:8006/api2/json
APP__PROXMOX__AUTH_HEADER=PVEAPIToken=dashboard_svc@pve!dashboard_token=e7b79db5-c725-486f-ace9-27295f96f44c
```

With these steps completed, your application is now configured to securely communicate with the Proxmox testbed.
