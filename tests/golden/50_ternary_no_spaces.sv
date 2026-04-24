module m;
  assign x=a?b:c;
endmodule
// expected -----
module m;
  assign x = a ? b : c;
endmodule
