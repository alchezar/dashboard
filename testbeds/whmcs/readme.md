# WHMCS Testbed Setup

This document outlines the steps to set up a local WHMCS instance using Docker. This testbed simulates the legacy system and serves as the data source for the migration utility, as detailed in the main project plan. For official documentation, please refer to the [WHMCS Documentation](https://docs.whmcs.com/8-13/).

## 1. Prerequisites

Before you begin, ensure you have the following installed on your system:

* [Docker](https://docs.docker.com/get-docker/)
* [Docker Compose](https://docs.docker.com/compose/install/)

## 2. Downloading Required Files

This setup requires manual download of the WHMCS core files and the Proxmox VE addon.

### 2.1. Download WHMCS

[Download](https://workupload.com/start/e9ZPyTpcutW) your WHMCS installation files (v8.8.0 or compatible). You should have an archive file, for example `WHMCS v8.8.0.rar`.

### 2.2. Download Proxmox VE for WHMCS Plugin

Download the Proxmox VE integration module from the [official GitHub repository]( https://github.com/The-Network-Crew/Proxmox-VE-for-WHMCS). Choose the latest release (e.g., `pvewhmcs-v1-2-15.zip`) from the releases page.

## 3. Environment Setup

This section covers placing the files and configuring the Docker environment.

### 3.1. Place WHMCS Files

* Create a directory named `whmcs` in the current folder.
* Extract the contents of your WHMCS archive into the newly created `whmcs` directory.

### 3.2. Place Plugin Files

* Extract the plugin archive (e.g., `pvewhmcs-v1-2-15.zip`).
* Inside the extracted folder, you will find the module files. Copy these files into the WHMCS addon directory at:
  `whmcs/modules/addons/pvewhmcs/`. You may need to create the `pvewhmcs` folder.

### 3.3. Configure Environment Variables

The Docker environment is configured using a `.env` file. This file defines the database credentials. You need to have a `.env` file with the following content:

```dotenv
# MySQL Database Settings
WHMCS_DB_NAME=whmcs_db_v8
WHMCS_DB_USER=whmcs_user
WHMCS_DB_PASS=your_strong_password
WHMCS_DB_ROOT_PASS=your_very_strong_root_password
```

### 3.4. Start the Docker Environment

Once all files are in place and the `.env` file is configured, you can start the services.

* Open a terminal in this directory.
* Run the following command:

```shell
docker-compose up -d
```

This command will build and start the `mysql_db` and `web8` (WHMCS) containers.

## 4. WHMCS Installation

With the environment running, you will perform the web-based installation and then import the provided SQL dump to seed the database.

### 4.1. Run the Web-Based Installer

* Navigate to `http://localhost/install/install.php` in your web browser.
* Follow the on-screen instructions. When prompted for database details, use the credentials from your `.env` file.
* Complete the installation and create your administrator account.

> **Note:** After installation, delete the `install` directory from the `whmcs/` folder.

### 4.2. Import Sample Data

The provided SQL dump (`whmcs_mysql_dump.sql`) will seed your installation with the required sample data, including clients, products, and the Proxmox module configuration.

Execute the following command to import the data:

```shell
docker-compose exec -T mysql_db mysql -u root -p${WHMCS_DB_ROOT_PASS} ${WHMCS_DB_NAME} < whmcs_mysql_dump.sql
```

## 5. Final Verification

* Log in to the WHMCS admin panel at `http://localhost/admin/`.
* Navigate to `System Settings > Addon Modules`.
* Verify that "Proxmox VE for WHMCS" is listed and activated.
* Navigate to `Clients > View/Search Clients` and confirm that the sample clients exist.
* Check a client's `Products/Services` to ensure they have `Configurable Options` and `Custom Fields`.

The testbed is now fully configured and ready to serve as the source for the data migration utility.
