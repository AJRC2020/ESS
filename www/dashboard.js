document.addEventListener("DOMContentLoaded", () => {
    function selectAllText(input) {
        input.setSelectionRange(0, input.value.length);
    }

    function copyText(input) {
        input.select();
        try {
            navigator.clipboard.writeText(input.value);
            alert("Copied to clipboard!");
        } catch (err) {
            console.error("Unable to copy:", err);
            alert("Unable to copy to clipboard. Please try manually.");
        }
    }

    const listUpload = document.getElementById("fileList");
    const token = localStorage.getItem("jwtToken");
    const private_key = localStorage.getItem("privateKey");

    function canViewFileContent(fileName) {
        const fileExtension = fileName.split(".").pop().toLowerCase();
        return fileExtension === "txt";
    }

    // Function to create a list item with buttons for each file
    function createListItem(fileName) {
        const listItem = document.createElement("li");
        listItem.textContent = fileName;
        listItem.style.margin = 10;

        const downloadBtn = createDownloadButton(fileName);
        const shareBtn = createShareButton(fileName);
        const shareLinksBtn = createViewShareLinksButton(fileName);

        if (canViewFileContent(fileName)) {
            const readBtn = createReadButton(fileName);
            listItem.appendChild(readBtn);
        }
        listItem.appendChild(downloadBtn);
        listItem.appendChild(shareBtn);
        listItem.appendChild(shareLinksBtn);

        return listItem;
    }

    function createReadButton(fileName) {
        const readBtn = document.createElement("button");
        readBtn.classList = "readBtn";
        readBtn.textContent = "View Content";
        readBtn.style.marginRight = 10;

        readBtn.addEventListener("click", () => {
            readFile(fileName);
        });

        return readBtn;
    }

    function createShareButton(fileName) {
        const shareBtn = document.createElement("button");
        shareBtn.classList = "shareBtn";
        shareBtn.textContent = "Create Share Link";

        shareBtn.addEventListener("click", () => {
            shareFile(fileName);
        });

        return shareBtn;
    }

    function createViewShareLinksButton(fileName) {
        const shareLinksBtn = document.createElement("button");
        shareLinksBtn.classList = "shareLinksBtn";
        shareLinksBtn.textContent = "Show Share Links";
        shareLinksBtn.dataset.fileName = fileName; // Attach the file name as a data attribute

        return shareLinksBtn;
    }

    function createDownloadButton(fileName) {
        const downloadBtn = document.createElement("button");
        downloadBtn.classList = "downloadBtn";
        downloadBtn.textContent = "Download";

        downloadBtn.addEventListener("click", () => {
            downloadFile(fileName);
        });

        return downloadBtn;
    }

    function shareFile(fileName) {
        const xhr = new XMLHttpRequest();

        const data = { file_name: fileName }; // Prepare the data in JSON format

        xhr.onreadystatechange = function () {
            if (xhr.readyState === XMLHttpRequest.DONE) {
                if (xhr.status === 200) {
                    const shareableLink = xhr.responseText;
                    alert(`Shareable link created: ${shareableLink}`);
                    // Handle the link as needed (e.g., displaying it to the user)
                } else if (xhr.status === 403) {
                    alert("You don't have permission to create links.");
                } else {
                    console.error(
                        "Failed to create shareable link:",
                        xhr.statusText
                    );
                }
            }
        };

        timestamp = getCurrentTimestamp();
        path = "https://localhost:8080/link";
        //message = timestamp + "+" + path + "+" + data;
        message = timestamp + "+" + path;
        hash = signWithPrivateKey(message);

        xhr.open("PUT", path);
        if (token) {
            xhr.setRequestHeader("Authorization", `Bearer ${token}`);
        }

        xhr.setRequestHeader("Hash", hash);
        xhr.setRequestHeader("Timestamp", timestamp);

        // Set the content type to application/json
        xhr.setRequestHeader("Content-Type", "application/json");

        xhr.send(JSON.stringify(data)); // Send the JSON data

        fetchAllLinks();
    }

    function readFile(fileName) {
        if (!canViewFileContent(fileName)) {
            displayFileContentInModal(
                "Unexpected error. File extension is not supported for viewing. Please download the file."
            );
            return;
        }

        const xhr = new XMLHttpRequest();

        xhr.onreadystatechange = function () {
            if (xhr.readyState === XMLHttpRequest.DONE) {
                if (xhr.status === 200) {
                    const fileContent = xhr.responseText;
                    displayFileContentInModal(fileContent);
                } else {
                    console.error("Failed to fetch file:", xhr.statusText);
                }
            }
        };

        timestamp = getCurrentTimestamp();
        path = `https://localhost:8080/files/${encodeURIComponent(fileName)}`;
        //message = path.concat("+", timestamp);
        message = timestamp + "+" + path;
        hash = signWithPrivateKey(message);

        xhr.open("GET", path);
        if (token) {
            xhr.setRequestHeader("Authorization", `Bearer ${token}`);
        }

        xhr.setRequestHeader("Hash", hash);
        xhr.setRequestHeader("Timestamp", timestamp);
        xhr.send();
    }

    function downloadFile(fileName) {
        const xhr = new XMLHttpRequest();

        xhr.onreadystatechange = function () {
            if (xhr.readyState === XMLHttpRequest.DONE) {
                if (xhr.status === 200) {
                    const blob = new Blob([xhr.response], {
                        type: xhr.getResponseHeader("Content-Type"),
                    });
                    const url = URL.createObjectURL(blob);

                    const a = document.createElement("a");
                    a.style.display = "none";
                    a.href = url;
                    a.download = fileName;

                    document.body.appendChild(a);
                    a.click();

                    window.URL.revokeObjectURL(url);
                    document.body.removeChild(a);
                } else {
                    console.error("Failed to fetch file:", xhr.statusText);
                }
            }
        };

        timestamp = getCurrentTimestamp();
        path = `https://localhost:8080/files/${encodeURIComponent(fileName)}`;
        message = timestamp + "+" + path;
        hash = signWithPrivateKey(message);

        xhr.open("GET", path);
        if (token) {
            xhr.setRequestHeader("Authorization", `Bearer ${token}`);
        }
        xhr.setRequestHeader("Hash", hash);
        xhr.setRequestHeader("Timestamp", timestamp);
        xhr.responseType = "blob";
        xhr.send();
    }

    // Fetch files from the server and update the fileList
    function fetchFiles() {
        const xhr = new XMLHttpRequest();
        xhr.onreadystatechange = () => {
            if (xhr.readyState === XMLHttpRequest.DONE) {
                if (xhr.status === 200) {
                    const files = xhr.responseText;

                    const fileList = JSON.parse(files);

                    listUpload.innerHTML = "";

                    fileList.forEach((fileName) => {
                        const listItem = createListItem(fileName);
                        listUpload.appendChild(listItem);
                    });

                    //fetch of links for files
                    fetchAllLinks();
                } else {
                    const error = xhr.responseText;
                    console.error("Error fetching files:", error);
                }
            }
        };

        const timestamp = getCurrentTimestamp();
        const path = "https://localhost:8080/files";

        message = timestamp + "+" + path;
        hash = signWithPrivateKey(message);
        xhr.open("GET", path);
        if (token) {
            xhr.setRequestHeader("Authorization", `Bearer ${token}`);
        }

        xhr.setRequestHeader("Hash", hash);
        xhr.setRequestHeader("Timestamp", timestamp);

        xhr.send();
    }

    function fetchAllLinks() {
        const xhr = new XMLHttpRequest();

        xhr.onreadystatechange = function () {
            if (xhr.readyState === XMLHttpRequest.DONE) {
                if (xhr.status === 200) {
                    const allLinks = JSON.parse(xhr.responseText);
                    const shareLinksBtns = document.querySelectorAll(
                        "[class='shareLinksBtn']"
                    );

                    shareLinksBtns.forEach((btn) => {
                        btn.addEventListener("click", () => {
                            const fileName = btn.dataset.fileName;
                            const linksWithSameFileName = Object.entries(
                                allLinks
                            ).filter(([key, link]) => {
                                return link.file_name === fileName;
                            });

                            displayShareLinksModal(linksWithSameFileName);
                        });
                    });
                } else {
                    console.error("Failed to fetch all links:", xhr.statusText);
                }
            }
        };

        const timestamp = getCurrentTimestamp();
        const path = "https://localhost:8080/links";

        message = timestamp + "+" + path;
        hash = signWithPrivateKey(message);

        xhr.open("GET", path);
        if (token) {
            xhr.setRequestHeader("Authorization", `Bearer ${token}`);
        }

        xhr.setRequestHeader("Hash", hash);
        xhr.setRequestHeader("Timestamp", timestamp);
        xhr.send();
    }

    function displayFileContentInModal(content) {
        const modal = document.getElementById("fileModal");
        const fileContentDisplay =
            document.getElementById("fileContentDisplay");

        // Update the content of the fileContentDisplay element with the file content
        fileContentDisplay.textContent = content;

        // Show the modal
        modal.style.display = "block";

        // Get the close button
        const closeButton = document.getElementById("close-btn-modal-content");

        // When the user clicks on the close button, hide the modal
        closeButton.onclick = function () {
            modal.style.display = "none";
        };

        // When the user clicks anywhere outside of the modal, close it
        window.onclick = function (event) {
            if (event.target === modal) {
                modal.style.display = "none";
            }
        };
    }

    function displayShareLinksModal(shareLinks) {
        const modal = document.getElementById("linksModal");
        const linksContentDisplay = document.getElementById(
            "LinksContentDisplay"
        );

        linksContentDisplay.innerHTML = "";

        // Populate the content area with share links
        shareLinks.forEach((link) => {
            const linkItem = document.createElement("div");
            linkItem.classList.add("link-box");

            const linkInput = document.createElement("input");
            linkInput.type = "text";
            linkInput.id = "link-" + link[0];
            linkInput.style.width = "40%";
            linkInput.value = window.location.origin + "/link/" + link[0];
            linkInput.style.marginRight = 10;
            linkInput.readOnly = true;
            linkInput.onclick = function () {
                selectAllText(this);
            };
            linkItem.appendChild(linkInput);

            // Create a copy button
            const copyBtn = document.createElement("button");
            copyBtn.textContent = "Copy";
            copyBtn.style.marginRight = 10;
            copyBtn.onclick = function () {
                copyText(linkInput);
            };
            linkItem.appendChild(copyBtn);

            // Create a delete button
            const deleteBtn = document.createElement("button");
            deleteBtn.textContent = "Delete Link";
            deleteBtn.classList.add("deleteBtn"); // Adding a class for easier selection later
            deleteBtn.style.backgroundColor = "#a80000";
            deleteBtn.style.color = "white";
            linkItem.appendChild(deleteBtn);

            linksContentDisplay.appendChild(linkItem);

            deleteBtn.addEventListener("click", () => {
                deleteLink(link[0]); // Pass the link text to the function to delete
            });
        });

        // Show the modal
        modal.style.display = "block";

        // Get the close button
        const closeButton = document.getElementById("close-btn-modal-links");

        // When the user clicks on the close button, hide the modal
        closeButton.onclick = function () {
            modal.style.display = "none";
        };

        // When the user clicks anywhere outside of the modal, close it
        window.onclick = function (event) {
            if (event.target === modal) {
                modal.style.display = "none";
            }
        };
    }

    function copyToClipboard(text) {
        navigator.clipboard
            .writeText(text)
            .then(() => {
                alert("Link copied to clipboard");
            })
            .catch((err) => {
                alert("Error copying Link to clipboard");
            });
    }

    function deleteLink(linkText) {
        const xhr = new XMLHttpRequest();

        xhr.onreadystatechange = function () {
            if (xhr.readyState === XMLHttpRequest.DONE) {
                if (xhr.status === 200) {
                    alert("Link deleted successfully");
                    removeDeletedLinkFromModal(linkText);
                    fetchFiles(); // Fetch updated files after deletion
                } else {
                    console.error("Failed to delete link:", xhr.statusText);
                }
            }
        };

        const timestamp = getCurrentTimestamp();
        const path = `https://localhost:27401/link/${encodeURIComponent(
            linkText
        )}`;

        message = timestamp + "+" + path;
        console.log(message);
        hash = signWithPrivateKey(message);

        xhr.open("DELETE", path);
        if (token) {
            xhr.setRequestHeader("Authorization", `Bearer ${token}`);
        }

        xhr.setRequestHeader("Hash", hash);
        xhr.setRequestHeader("Timestamp", timestamp);
        xhr.send();
    }

    function removeDeletedLinkFromModal(linkText) {
        const linksContentDisplay = document.getElementById(
            "LinksContentDisplay"
        );
        const links = linksContentDisplay.querySelectorAll(".link-box");
        links.forEach((link) => {
            const linkInput = link.querySelector("input");
            if (linkInput.id === "link-" + linkText) {
                link.remove();
            }
        });
    }

    function getCurrentTimestamp() {
        return new Date().getTime().toString();
    }

    function signWithPrivateKey(inputValue) {
        var sign = new JSEncrypt();
        sign.setPrivateKey(private_key);

        var signature = sign.sign(inputValue, CryptoJS.SHA256, "sha256");
        return signature;
    }

    // Initial fetch of files
    fetchFiles();

    // Handle file upload
    const uploadBtn = document.getElementById("uploadBtn");
    uploadBtn.addEventListener("click", () => {
        const fileInput = document.getElementById("fileUpload");
        const file = fileInput.files[0];

        if (file) {
            const reader = new FileReader();
            reader.onload = () => {
                const fileContent = reader.result;
                const formData = new FormData();
                formData.append("contents", fileContent);

                const xhr = new XMLHttpRequest();
                xhr.onreadystatechange = () => {
                    if (xhr.readyState === XMLHttpRequest.DONE) {
                        if (xhr.status === 200) {
                            alert("File uploaded!");
                            fetchFiles();
                        } else if (xhr.status === 403) {
                            alert("You don't have permission to upload files.");
                        } else {
                            const error = xhr.responseText;
                            alert(`File upload failed: ${error}`);
                        }
                    }
                };

                const timestamp = getCurrentTimestamp();
                const path = `https://localhost:8080/files/${encodeURIComponent(
                    file.name
                )}`;

                message = timestamp + "+" + path;
                hash = signWithPrivateKey(message);

                xhr.open("PUT", path);
                if (token) {
                    xhr.setRequestHeader("Authorization", `Bearer ${token}`);
                }
                xhr.setRequestHeader("Hash", hash);
                xhr.setRequestHeader("Timestamp", timestamp);

                xhr.send(formData);
            };
            reader.readAsText(file);
        } else {
            alert("Please select a file to upload");
        }
    });

    const logoutBtn = document.getElementById("logoutBtn");
    logoutBtn.addEventListener("click", () => {
        localStorage.removeItem("jwtToken");
        localStorage.removeItem("privateKey");
        window.location.href = "/login.html";
    });
});
