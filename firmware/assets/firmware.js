function uploadFirmware(file) {
    if (!file) return;

    // Create or get the loading indicator
    let indicator = document.getElementById('firmware-upload-indicator');
    if (!indicator) {
        indicator = document.createElement('div');
        indicator.id = 'firmware-upload-indicator';
        indicator.style.marginTop = '10px';
        indicator.style.fontWeight = 'bold';
        document.getElementById('firmware-upload').parentNode.appendChild(indicator);
    }
    indicator.textContent = 'Uploading... 0%';
    indicator.style.display = 'block';

    const xhr = new XMLHttpRequest();
    xhr.open('POST', '/upload-firmware', true);

    xhr.upload.onprogress = function (event) {
        if (event.lengthComputable) {
            const percent = Math.round((event.loaded / event.total) * 100);
            indicator.textContent = `Uploading... ${percent}%`;
        } else {
            indicator.textContent = 'Uploading...';
        }
    };

    xhr.onload = function () {
        if (xhr.status === 200) {
            indicator.textContent = 'Upload complete! ' + xhr.responseText;
            setTimeout(() => { indicator.style.display = 'none'; }, 4000);
        } else {
            indicator.textContent = 'Firmware upload failed. Please try again.';
        }
    };

    xhr.onerror = function () {
        indicator.textContent = 'Firmware upload failed. Please try again.';
    };

    // Read the file as an ArrayBuffer and send it directly
    const reader = new FileReader();
    reader.onload = function (e) {
        xhr.send(e.target.result);
    };
    reader.readAsArrayBuffer(file);
} 