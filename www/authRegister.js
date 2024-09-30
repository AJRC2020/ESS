document.addEventListener("DOMContentLoaded", function () {
    const registerForm = document.getElementById("register-form");
    const passwordError = document.getElementById("password-error");
    const errorMessageContainer = document.getElementById("error-message");
    const infoMessageContainer = document.getElementById("info-message");
    const submitButton = document.getElementById('submitButton');

    const passwordField = document.getElementById('password');
    const confirmPasswordField = document.getElementById('confirm-password');
    const showPasswordButton = document.getElementById('show-password');

    showPasswordButton.addEventListener('click', function() {
        if (passwordField.type === 'password') {
            passwordField.type = 'text';
            confirmPasswordField.type = 'text';
            showPasswordButton.textContent = 'Hide Password';
        } else {
            passwordField.type = 'password';
            confirmPasswordField.type = 'password';
            showPasswordButton.textContent = 'Show Password';
        }
    });

    function generatePassword() {
        const length = 16;
        const charset = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+~`|}{[]\:;?><,./-=';
        let retVal = '';
        for (let i = 0, n = charset.length; i < length; ++i) {
            retVal += charset.charAt(Math.floor(Math.random() * n));
        }
        // force toggle show password on
        passwordField.type = 'text';
        confirmPasswordField.type = 'text';
        showPasswordButton.textContent = 'Hide Password';
        return retVal;
    }

    const generatePasswordButton = document.getElementById('generate-password');
    generatePasswordButton.addEventListener('click', function() {
        passwordField.value = generatePassword();
        confirmPasswordField.value = passwordField.value;
    });


    function handleRegistrationError(error) {
        infoMessageContainer.style.display = "none";
        errorMessageContainer.textContent = error.message;
        errorMessageContainer.style.display = "block";
        submitButton.disabled = false;
    }

    // Function to handle user registration response
    function handleRegistrationResponse(response, formData) {
        if (!response.ok) {
            if (response.status === 422) {
                throw new Error("Username contains invalid characters");
            }
            else if (response.status === 409 || response.status === 400) {
                return response.json().then(errorMsg => {
                    const customError = new Error(errorMsg.error ? ("Problem creating account: "+errorMsg.error) : "An error occurred. Please try again later.");
                    throw customError;
                });
            }
            throw new Error("An error occurred. Please try again later.");
        }

        //Login user immediatly
        loginUserAfterRegister(formData);
    }
    // Function to handle user registration
    function registerUser(formData) {
        console.log("Sending registration request...")

        fetch("https://localhost:27464/user/register", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify(formData),
        })
            .then((response) => handleRegistrationResponse(response, formData))
            .catch((error) => handleRegistrationError(error));
    }

    function loginUserAfterRegister(formData) {
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
                    throw new Error("Login failed");
                }
            })
            .then((data) => {
                localStorage.setItem("privateKey", data.private_key);
                localStorage.setItem("jwtToken", data.token);
                const username = document.getElementById("username").value;
                window.location.href = `dashboard.html?username=${username}`;
            })
            .catch((error) => {
                console.error(error);
                submitButton.disabled = false;
            });
    }

    // Event listener for register form submission
    registerForm.addEventListener("submit", function (event) {
        event.preventDefault();

        //check password repetition

        const password = document.getElementById("password").value;
        const confirmPassword =
            document.getElementById("confirm-password").value;

        if (password !== confirmPassword) {
            passwordError.style.display = "block";
            return;
        }

        passwordError.style.display = "none";
        errorMessageContainer.style.display = "none";

        submitButton.disabled = true;

        infoMessageContainer.textContent = "Please wait...";
        infoMessageContainer.style.display = "block";

        const formData = new FormData(registerForm);
        const formDataObject = {};
        formData.forEach((value, key) => {
            formDataObject[key] = value;
        });
        registerUser(formDataObject);
    });
});
