use reqwest;
use serde_json;
use appointment::*;
use std::io::Read;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct ScheduleError(&'static str);

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl Error for ScheduleError {
    fn description(&self) -> &str {
        self.0
    }
}

/// This struct represents the schedule, containing [Appointment](./struct.Appointment.html)s.
pub struct Schedule {
    /// The school id used in the URL.
    /// For the school id of 'example', this URL will be: `https://example.zportal.nl/`.
    pub school: String,
    /// The access token obtained from the API, used to obtain appointments.
    pub access_token: String,
    /// A vector of the appointments.
    pub appointments: Vec<Appointment>,
}

impl Schedule {
    /// Create a new `Schedule` from an authorization code (only once usable) and a school identifier.
    /// This will get the access token from the API.
    /// Returns a `Schedule` or an error.
    pub fn new<S>(school: &S, code: &S) -> Result<Self, Box<Error>>
    where
        S: ToString,
    {
        let school = school.to_string();
        let code = code.to_string();

        let url = format!("https://{}.zportal.nl/api/v3/oauth/token", school);
        // Remove spaces from code.
        let code = code.replace(" ", "");
        let post_data = [("grant_type", "autorization_code"), ("code", code.as_str())];

        // Send request.
        let mut response = reqwest::Client::new()
            .post(url.as_str())
            .form(&post_data)
            .send()?;

        // Check whether response code equals "200 OK".
        if response.status().as_u16() != 200 {
            return Err(Box::new(ScheduleError("response code is not 200")));
        }

        // Parse response as JSON.
        let json: AuthenticationResponse = response.json()?;

        let access_token = json.access_token;

        Ok(Schedule {
            school: school.to_owned(),
            access_token,
            appointments: Vec::new(),
        })
    }

    /// Create a new `Schedule` when an access token has been obtained already.
    /// This cannot fail, so this will not return a `Result`.
    pub fn with_access_token<S>(school: &S, access_token: &S) -> Self
    where
        S: ToString,
    {
        Schedule {
            school: school.to_string(),
            access_token: access_token.to_string(),
            appointments: Vec::new(),
        }
    }

    /// Get the appointments between `start` and `end` from the API, and set them to `self.appointments`.
    /// Returns a reference to itself, or an error.
    pub fn get_appointments(&mut self, start: i64, end: i64) -> Result<&Self, Box<Error>> {
        let url = format!(
            "https://{}.zportal.nl/api/v3/appointments?user=~me&start={}&end={}&access_token={}",
            self.school, start, end, self.access_token
        );

        // Make request.
        let mut response = reqwest::get(url.as_str())?;

        // Check whether response code equals "200 OK".
        if response.status().as_u16() != 200 {
            return Err(Box::new(ScheduleError("response code is not 200")));
        }

        // Read body to string.
        let mut body = String::new();
        response.read_to_string(&mut body)?;

        // Replace camelCase index with snake_case index, so we can deserialize it easier.
        let body = body.replace("appointmentInstance", "appointment_instance")
            .replace("startTimeSlot", "start_time_slot")
            .replace("endTimeSlot", "end_time_slot")
            .replace("type", "appointment_type")
            .replace("lastModified", "last_modified")
            .replace("changeDescription", "change_description")
            .replace("branchOfSchool", "branch_of_school");

        let response: AppointmentsResponse = serde_json::from_str(body.as_str())?;

        self.appointments = response.response.data;

        // Sort appointments by start time.
        self.appointments
            .sort_unstable_by_key(|k| k.start.unwrap_or(0));

        Ok(self)
    }
}

#[derive(Deserialize)]
struct AuthenticationResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct AppointmentsResponse {
    response: AppointmentsResponseResponse,
}

#[derive(Deserialize)]
// Why, Zermelo, would you wrap everything in a "response" map?
struct AppointmentsResponseResponse {
    data: Vec<Appointment>,
}

#[cfg(test)]
mod tests {
    use serde_json;
    use schedule::*;

    #[test]
    fn parse_request() {
        // Data example from https://zermelo.atlassian.net/wiki/spaces/DEV/pages/22577247/Example+Retrieving+a+Schedule.
        let json = r#"{
            "response": {
                "status": 200,
                "message": "",
                "startRow": 0,
                "endRow": 27,
                "totalRows": 27,
                "data": [
                    {
                        "id": 5,
                        "start": 42364236,
                        "end": 436234523,
                        "startTimeSlot": 1,
                        "endTimeSlot": 1,
                        "subjects": ["ne"],
                        "teachers": ["KRO"],
                        "groups": ["v1a"],
                        "locations": ["M92"],
                        "type": "lesson",
                        "remark": "Take care to bring your books",
                        "valid": true,
                        "cancelled": false,
                        "modified": true,
                        "moved": false,
                        "new": false,
                        "changeDescription": "The location has been changed from M13 to M92"
                    }
                ]
            }
        }"#;

        let json = json.replace("appointmentInstance", "appointment_instance")
            .replace("startTimeSlot", "start_time_slot")
            .replace("endTimeSlot", "end_time_slot")
            .replace("type", "appointment_type")
            .replace("lastModified", "lastModified")
            .replace("changeDescription", "change_description")
            .replace("branchOfSchool", "branch_of_school");

        let response: AppointmentsResponse = serde_json::from_str(json.as_str()).unwrap();
        let appointment = &response.response.data[0];
        assert_eq!(appointment.id, Some(5));
        assert_eq!(appointment.start, Some(42364236));
        assert_eq!(appointment.start_time_slot, Some(1));
        assert_eq!(appointment.subjects, Some(vec![String::from("ne")]));
        assert_eq!(appointment.appointment_type, Some(String::from("lesson")));
        assert_eq!(appointment.cancelled, Some(false));
    }
}
