module m; // top comment
  assign a = 1; // trailing
  assign b = 2;// tight
endmodule
// expected -----
module m; // top comment
  assign a = 1; // trailing
  assign b = 2; // tight
endmodule
