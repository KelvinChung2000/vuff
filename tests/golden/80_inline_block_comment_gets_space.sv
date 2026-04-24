module m;
  assign b = 2;/*inline*/
  assign c = 3;/*before end*/ // trailing
endmodule
// expected -----
module m;
  assign b = 2; /*inline*/
  assign c = 3; /*before end*/ // trailing
endmodule
