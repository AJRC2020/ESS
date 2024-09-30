document.addEventListener("DOMContentLoaded", function () {
    const token = localStorage.getItem("jwtToken");

    if (token) {
        // Use the token to authenticate
        authenticateWithToken(token);
    } else {
        // Redirect to the login page if no token is found
        window.location.href = "/login.html";
    }

    function authenticateWithToken(token) {
        const claims = jose.decodeJwt(token);
        // Get the current time in seconds
        let now = Math.floor(Date.now() / 1000);
        // Get the exp field from the claims
        let exp = claims.exp;
        // Calculate the difference in seconds
        let diff = now - exp;

        if (diff > 0) {
            window.location.href = "/login.html";
        } else {
            window.location.href = "/dashboard.html";
        }
    }
});
