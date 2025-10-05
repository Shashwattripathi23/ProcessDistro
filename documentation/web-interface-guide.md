# ProcessDistro Web Interface - Device Management

## Overview

The ProcessDistro web interface now supports device registration, unregistration, and automatic cleanup when devices leave the network.

## Features Added

### 1. Device Registration & Unregistration

- **Register Device**: Devices can register to join the distributed computing network
- **Unregister Device**: Devices can manually unregister to leave the network
- **Automatic Cleanup**: Devices are automatically removed when they close the browser, reload the page, or lose connection

### 2. Access from Multiple Devices

#### To access from the host machine:

- Open browser and go to: `http://localhost:30100`

#### To access from other devices on the same network:

- Ensure all devices are connected to the same WiFi network
- On other devices, open a web browser
- Go to: `http://192.168.1.7:30100` (replace with your actual IP)

### 3. Device Capabilities Detection

The web interface automatically detects and displays:

- **CPU Cores**: Number of processing cores available
- **Memory**: Total and available memory
- **GPU Information**: Graphics processing capabilities (if available)
- **Browser**: Browser type and version
- **Platform**: Operating system information
- **Benchmark Score**: Performance score after running benchmark

### 4. Network Management

- **Real-time Node List**: View all connected devices and their status
- **Performance Metrics**: Total computing power across all nodes
- **Connection Status**: Monitor device connectivity and health

## Usage Instructions

### For Device Owners:

1. **Join the Network**:

   - Open the web interface URL
   - Click "Register This Device"
   - Optionally run a benchmark to measure performance

2. **Leave the Network**:

   - Click "Unregister Device" to manually leave
   - Or simply close the browser tab (automatic cleanup)

3. **Monitor Performance**:
   - View your device capabilities
   - See benchmark results
   - Monitor connection status

### For Network Administrators:

1. **Start the Web Server**:

   ```bash
   cd processdistro
   cargo run --bin web-server
   ```

2. **Monitor the Network**:

   - View all connected devices
   - Check total computing resources
   - Monitor device health and status

3. **Share Connection Info**:
   - The server displays the network IP for other devices
   - Share the URL with team members
   - Use the QR code (if implemented) for easy mobile access

## Technical Details

### API Endpoints

- `GET /` - Main dashboard interface
- `GET /api/nodes` - Get list of all connected nodes
- `POST /api/register` - Register a new device
- `POST /api/unregister` - Unregister a device
- `POST /api/heartbeat` - Device health check
- `POST /api/benchmark` - Submit benchmark results

### Automatic Cleanup

The system handles device disconnection through multiple mechanisms:

- **beforeunload**: Triggered when user closes tab or navigates away
- **visibilitychange**: Triggered when page becomes hidden
- **pagehide**: Triggered when page is unloaded
- **sendBeacon**: Ensures cleanup requests reach server even during rapid disconnection

### Device Status

- **Active**: Device is connected and ready for tasks
- **Idle**: Device is connected but not currently processing
- **Offline**: Device has disconnected or is unresponsive

## Network Requirements

- All devices must be on the same local network (WiFi/Ethernet)
- Port 30100 must be accessible between devices
- Modern web browser with JavaScript enabled
- WebAssembly support for benchmark execution

## Security Considerations

- The web interface is designed for local network use only
- No authentication is required (suitable for trusted networks)
- Device IDs are automatically generated and ephemeral
- No persistent data storage on devices

## Troubleshooting

### Device Can't Connect

1. Verify both devices are on the same network
2. Check if firewall is blocking port 30100
3. Ensure the web server is running
4. Try accessing from the host machine first

### Device Not Showing in List

1. Check if registration was successful
2. Refresh the page to reload node list
3. Verify device is still connected to network
4. Check browser console for error messages

### Automatic Cleanup Not Working

1. Some browsers may delay cleanup events
2. Devices will be marked offline after missing heartbeats
3. Manual unregistration is always reliable
4. Server logs will show cleanup attempts
