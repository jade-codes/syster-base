package CargoDrone {

 package 'Problem Space' {

 // ==== Item types (what flows) ====

 item def Control;
 item def Indication;
 item def Command;


 // ==== System of Interest ====

 part def 'Cargo Drone' {
 in commandIn : Command;
 out stateOut : Indication;
 }


 // ==== External Systems / Actors ====

 part def 'Drone Operation System' {
 in controlFromOperator : Control;
 out indicationToOperator : Indication;

 out commandToDrone : Command;
 in stateFromDrone : Indication;
 }

 part def 'Drone Safety Operator' {
 out controlOut : Control;
 in indicationIn : Indication;
 }


 // ==== Context Structure (Interconnection View) ====

 part 'Cargo Drone System Context' {

 part drone : 'Cargo Drone';
 part ops : 'Drone Operation System';
 part safety : 'Drone Safety Operator';


 // Safety Operator ↔ Operation System

 connection connect safety.controlOut
 to ops.controlFromOperator;

 connection connect ops.indicationToOperator
 to safety.indicationIn;


 // Operation System ↔ Drone

 connection connect ops.commandToDrone
 to drone.commandIn;

 connection connect drone.stateOut
 to ops.stateFromDrone;
 }
 }
}