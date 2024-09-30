document.addEventListener("DOMContentLoaded", function () {
    const loginForm = document.getElementById("login-form");
    const errorMessageContainer = document.getElementById("error-message");
    const infoMessageContainer = document.getElementById("info-message");
    const submitButton = document.getElementById('submitButton');

    function handleLoginError(error) {
        console.log(error);
        infoMessageContainer.style.display = "none";
        errorMessageContainer.textContent = error.message
        errorMessageContainer.style.display = "block";
        submitButton.disabled = false;
    }

    // Function to handle user login
    function loginUser(formData) {
        fetch("https://localhost:27464/user/login", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify(formData),
        })
            .then((response) => {
                // Check if the response is ok
                if (response.ok) {
                    return response.json();
                } else {
                    if (response.status === 400) {
                        return response.json().then(errorMsg => {
                            const customError = new Error(errorMsg.error ? ("Problem logging in: "+errorMsg.error) : "An error occurred. Please try again later.");
                            throw customError;
                        });
                    }
                    throw new Error("An error occurred. Please try again later.");
                }
            })
            .then((data) => {
                localStorage.setItem("privateKey", data.private_key);
                localStorage.setItem("jwtToken", data.token);
                window.location.href = `dashboard.html`;
            })
            .catch((error) => handleLoginError(error));
    }

    // Event listener for login form submission
    loginForm.addEventListener("submit", function (event) {
        event.preventDefault();

        console.log("Sending login request...")

        errorMessageContainer.style.display = "none";

        submitButton.disabled = true;

        infoMessageContainer.textContent = "Please wait...";
        infoMessageContainer.style.display = "block";

        const formData = new FormData(loginForm);
        const formDataObject = {};
        formData.forEach((value, key) => {
            formDataObject[key] = value;
        });
        loginUser(formDataObject);
    });
});
