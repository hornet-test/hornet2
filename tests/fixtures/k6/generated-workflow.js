// k6 script generated from Arazzo specification
// Contains 2 workflows

import http from 'k6/http';
import { check, sleep } from 'k6';

export let options = {
  vus: 1,
  iterations: 1,
};

// Workflow: user-onboarding-flow
// Complete user onboarding and profile setup
function user_onboarding_flow() {
  let inputs = {
    bio: "I am a test user",
    email: "test@example.com",
    newEmail: "updated@example.com",
    password: "securePassword123",
    username: "testuser",
  };

  // Step: register - Register a new user account
  let register_response = http.post("https://api.example.com/register", JSON.stringify({
  "email": inputs.email,
  "password": inputs.password,
  "username": inputs.username
}), {
    headers: {
      'Content-Type': "application/json"
    }
  });
  check(register_response, {
    'check_1': (r) => register_response.status === 201
  });
  let register_userId = register_response.json('id');
  let register_username = register_response.json('username');

  // Step: login - Login with the registered user
  let login_response = http.post("https://api.example.com/login", JSON.stringify({
  "password": inputs.password,
  "username": inputs.username
}), {
    headers: {
      'Content-Type': "application/json"
    }
  });
  check(login_response, {
    'check_1': (r) => login_response.status === 200
  });
  let login_token = login_response.json('token');
  let login_userId = login_response.json().user.id;

  // Step: getProfile - Retrieve the user profile
  let getProfile_response = http.get("https://api.example.com/profile", {
    headers: {
      'Authorization': `Bearer ${login_token}`
    }
  });
  check(getProfile_response, {
    'check_1': (r) => getProfile_response.status === 200,
    'check_2': (r) => getProfile_response.json('id') === register_userId
  });
  let getProfile_profile = getProfile_response.json();

  // Step: updateProfile - Update the user profile
  let updateProfile_response = http.put("https://api.example.com/profile", JSON.stringify({
  "bio": inputs.bio,
  "email": inputs.newEmail
}), {
    headers: {
      'Content-Type': "application/json",
      'Authorization': `Bearer ${login_token}`
    }
  });
  check(updateProfile_response, {
    'check_1': (r) => updateProfile_response.status === 200,
    'check_2': (r) => updateProfile_response.json('email') === inputs.newEmail
  });
  let updateProfile_updatedProfile = updateProfile_response.json();

}

// Workflow: simple-login-flow
// Simple login workflow
function simple_login_flow() {
  let inputs = {
    password: "securePassword123",
    username: "testuser",
  };

  // Step: login - Login user
  let login_response = http.post("https://api.example.com/login", JSON.stringify({
  "password": inputs.password,
  "username": inputs.username
}), {
    headers: {
      'Content-Type': "application/json"
    }
  });
  check(login_response, {
    'check_1': (r) => login_response.status === 200
  });
  let login_token = login_response.json('token');

}

export default function () {
  user_onboarding_flow();
  simple_login_flow();
  sleep(1);
}