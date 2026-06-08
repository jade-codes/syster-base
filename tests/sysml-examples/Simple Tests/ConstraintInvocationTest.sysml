package ConstraintInvocationTest {

	// Constraint definition with parameters
	constraint def AccuracyConstraint {
		in actual_accuracy : Real;
		in required_threshold : Real;

		actual_accuracy >= required_threshold
	}

	// Enum for noise types
	enum def EnvNoiseType {
		Rain;
		Wind;
		Clear;
	}

	// A requirement that uses if/then/else with constraint invocation body
	requirement def WeatherAccuracyReq {
		attribute threshold : Real;
		attribute envType : EnvNoiseType;
		attribute accuracy : Real;

		require constraint {
			if (envType == EnvNoiseType::Rain)
			then AccuracyConstraint {
				actual_accuracy = accuracy;
				required_threshold = threshold;
			}
			else true;
		}
	}

	// Simpler variant: constraint invocation body without if/then/else
	part def Sensor {
		attribute measuredAccuracy : Real;

		assert constraint : AccuracyConstraint {
			actual_accuracy = measuredAccuracy;
			required_threshold = 0.95;
		}
	}
}
