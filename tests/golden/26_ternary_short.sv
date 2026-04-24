module m;
  assign x=(a==b)?c:d;
  assign y = a?b:c;
endmodule
// expected -----
module m;
  assign x = (a == b) ? c : d;
  assign y = a ? b : c;
endmodule
